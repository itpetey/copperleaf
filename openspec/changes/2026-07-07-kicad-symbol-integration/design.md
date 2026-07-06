## Context

The schematic emitter (`crates/backends/kicad/src/schematic.rs`) generates a generic box symbol per component with pins evenly spaced at 2.54 mm on the right edge. KiCad's connection model is purely coordinate-based: a wire is "connected" to a pin only when their endpoints coincide. When a user changes the symbol in KiCad's GUI (Edit → Change Symbol), the replacement symbol has different pin geometry, so all wires become dangling.

The IR (`crates/ir/src/lib.rs`) has no fields for symbol references or pin positions. The `Pin` struct carries only logical data (name, role, limits, sig). The `ComponentRecord` carries refdes, pins, and constraints — nothing KiCad-specific.

The existing `part!` declarative macro (in `crates/edsl/src/lib.rs`) generates structs and `impl Block` from a pin table, but cannot set metadata like a KiCad symbol reference. ARCHITECTURE.md §8 names a future `#[derive(Component)]` macro with "metadata (symbol, footprint, pins)" — this change implements that vision.

There is no proc-macro crate in the workspace today. The sexpr module (`crates/backends/kicad/src/sexpr.rs`) is a builder/emitter, not a parser — it can write S-expressions but cannot read them.

## Goals / Non-Goals

**Goals:**
- Let part definitions carry a KiCad symbol library ID (e.g., `"RP2040:RP2354a"`).
- Let pins carry explicit positions so the schematic emitter places them correctly.
- Automatically populate pin positions by parsing `.kicad_sym` files.
- Generate schematics whose wires align with real KiCad symbols so no manual rewiring is needed after opening in KiCad.
- Provide a `#[derive(Component)]` proc macro as the ergonomic entry point.

**Non-Goals:**
- Automated footprint/pad position extraction from `.kicad_mod` or `.pretty` files (parallel concern, separate change).
- Embedding full KiCad symbol graphics (polylines, arcs, circles) in the generated schematic — we emit a simple rectangle body with correctly positioned pins; the real graphics come from KiCad's library when it resolves the `lib_id`.
- A build-time KiCad library index or dependency on KiCad's installation path — the user provides the `.kicad_sym` path.
- Replacing the `part!` declarative macro — both coexist; `part!` is for simple parts, `#[derive(Component)]` is for parts with metadata.

## Decisions

### D1: Pin position fields on `Pin` (IR)

**Decision:** Add `pos: Option<(f64, f64)>` and `rotation: Option<f64>` to `Pin`.

```rust
pub struct Pin {
    pub name: String,
    pub role: Role,
    pub limits: Limits,
    pub sig: Option<SigSpec>,
    pub pos: Option<(f64, f64)>,      // (x, y) in mm, relative to symbol origin
    pub rotation: Option<f64>,        // degrees (0, 90, 180, 270)
}
```

**Rationale:** Positions are logically part of the pin. `Option` keeps it backward-compatible — existing code that doesn't set them gets `None` and the emitter uses the algorithmic fallback.

**Migration:** `Pin::new()` sets `pos: None, rotation: None`. All existing `Pin { ... }` literals need the two new fields (or use `..Default::default()` if we impl `Default` for `Pin`). Since `Pin` derives `Clone, Debug` but not `Default`, we add `Default` (power-in pin with zero limits) or provide a builder pattern. Given the number of existing call sites, adding `Default` and updating literals is cleanest.

### D2: `kicad_symbol` on `ComponentRecord` (IR)

**Decision:** Add `kicad_symbol: Option<String>` to `ComponentRecord`.

```rust
pub struct ComponentRecord {
    pub refdes: String,
    pub pins: Vec<Pin>,
    pub constraints: Vec<Constraint>,
    pub kicad_symbol: Option<String>,
}
```

**Rationale:** The symbol reference is per-component, not per-pin. It flows from the `Block` impl (via a new trait method or attribute) through `ComponentInst` into `ComponentRecord` when `add_component` is called.

**How it gets populated:** The `Block` trait gains a default method `fn kicad_symbol(&self) -> Option<&str> { None }`. `Design::add_component` copies it into the `ComponentRecord` alongside pins and constraints.

### D3: `.kicad_sym` parser in the KiCad backend

**Decision:** Add a `sym_parser` module to `copperleaf-backend-kicad` that parses `.kicad_sym` files.

**Format:** A `.kicad_sym` file is an S-expression starting with `(kicad_symbol_lib ...)` containing `(symbol ...)` entries. Each symbol has `(pin ...)` nodes with `(at x y rot)`, `(name "...")`, and `(number "...")` children.

**Parser approach:** Add a `parse` function to the existing `sexpr` module (it currently only serializes). The parser tokenizes the S-expression text and builds a `Sexpr` tree. The `sym_parser` module then walks the tree to extract `SymbolDef` structs:

```rust
pub struct SymbolDef {
    pub lib_id: String,           // e.g. "RP2040:RP2354a"
    pub pins: Vec<PinDef>,
}

pub struct PinDef {
    pub name: String,
    pub number: String,
    pub pos: (f64, f64),
    pub rotation: f64,
    pub pin_type: String,         // "power_in", "bidirectional", etc.
    pub length: f64,              // pin stub length in mm
}
```

**Rationale:** We need the sexpr parser to read `.kicad_sym` files. Putting it in the sexpr module makes it available for future KiCad file ingestion (e.g., reading `.kicad_sch` files). The `sym_parser` module is backend-specific because it knows about KiCad symbol library semantics.

**Trade-off:** Writing a full S-expression parser is non-trivial but the grammar is simple (atoms, strings, lists, comments). We implement a minimal recursive-descent parser — not a full Lisp reader.

### D4: `resolve_symbols` pass

**Decision:** Add a function `resolve_symbols(design: &mut Design, lib_path: &str)` to the KiCad backend that:

1. Iterates `design.components`.
2. For each component where `kicad_symbol` is `Some(sym_id)` and any pin has `pos == None`:
   a. Parses the `.kicad_sym` file at `lib_path` (cached — parse once).
   b. Finds the symbol matching `sym_id`.
   c. For each pin in the component, finds the matching `PinDef` by name.
   d. Sets `pin.pos` and `pin.rotation` from the `PinDef`.

**Rationale:** This separates the "what symbol do I want?" declaration (in the part definition) from the "where are the pins?" resolution (in the backend). The user declares intent in the IR; the backend resolves it using KiCad's library files.

**Matching strategy:** Pin names from the IR are matched against `PinDef.name` from the symbol library. If names don't match (e.g., IR says `"VDD"` but the KiCad symbol says `"1"`), the user can also specify pin numbers on the IR pin (via a new optional `number: Option<String>` field, or via the proc macro attribute) to match by number instead.

**Failure handling:** If the symbol or a pin can't be found, emit a `Diagnostic` (warning) and fall back to algorithmic spacing for the unresolved pins. Do not panic — the schematic is still usable, just with possibly-dangling wires for that symbol.

### D5: Schematic emitter changes

**Decision:** Update `schematic.rs` to use per-pin positions when available.

- `lib_pin_node`: If `pin.pos` is `Some((x, y))`, emit `(at x y rotation)` instead of the algorithmic `(at 7.62 y_offset 180)`.
- `lib_symbol_for_component`: If `kicad_symbol` is `Some(sym_id)`, use it as the symbol name instead of `"copperleaf:{refdes}"`. The body rectangle stays the same (KiCad will replace it with real graphics from its library).
- `symbol_instance_node`: Use `kicad_symbol` for `lib_id` when present.
- `wire_node` / `label_node`: Compute pin tip coordinates from `pin.pos` and `pin.rotation` instead of the hardcoded `sym_x + 7.62, sym_y + y_off`.

**Pin tip calculation:** The pin tip is at `pos + length * direction(rotation)`. For rotation=0, tip is at `(x + length, y)`. For rotation=180, tip is at `(x - length, y)`. The default pin length is 2.54 mm unless the symbol library specifies otherwise.

**Rationale:** This makes the generated file internally consistent with real KiCad symbols. When KiCad resolves the `lib_id`, it finds the real symbol graphics and the wires already land on the correct pins.

### D6: `#[derive(Component)]` proc macro

**Decision:** Create a new `copperleaf-derive` crate (`[lib] proc-macro = true`) that provides `#[derive(Component)]`.

**Attribute syntax:**

```rust
use copperleaf_derive::Component;
use copperleaf_edsl::{Pin, Role, Limits, gnd, power_in, UnitExt};

#[derive(Clone, Debug, Component)]
#[component(symbol = "RP2040:RP2354a")]
pub struct Rp2354a {
    pins: Vec<Pin>,
}

impl Rp2354a {
    pub fn new() -> Self {
        Self {
            pins: vec![
                power_in(1.62.volt(), 3.6.volt(), 0.5.amp()).duplicate("VDD"),
                gnd().duplicate("GND"),
                // ... more pins
            ],
        }
    }
}
```

The derive generates `impl Block for Rp2354a` with `pins()` returning `&self.pins`, and `kicad_symbol()` returning `Some("RP2040:RP2354a")` from the `#[component(symbol = "...")]` attribute.

**Why a proc macro instead of extending `part!`:** The declarative `part!` macro can't easily express arbitrary struct shapes (extra fields for electrical params like `v_out`, `i_max` on `Buck`). A derive macro works on any struct shape — it only adds the trait impl, leaving the struct body to the user. This matches the existing `Buck`/`Mcu` pattern where structs have both `pins` and parameter fields.

**Crate structure:**
- `crates/derive/` → `copperleaf-derive` (proc-macro crate)
  - Depends on `syn`, `quote`, `proc-macro2` (workspace deps to add)
- `crates/edsl/` re-exports the derive: `pub use copperleaf_derive::Component;`
- `crates/copperleaf/` facade re-exports it transitively through `copperleaf-edsl`

**Alternative considered — attribute on `part!`:** We could add a `symbol = "..."` clause to the existing `part!` macro. But `part!` generates the struct, so it can't add extra fields for electrical parameters. Keep `part!` for simple passives; use `#[derive(Component)]` for ICs with metadata.

### D7: `Block` trait extension

**Decision:** Add `fn kicad_symbol(&self) -> Option<&str>` to the `Block` trait with a default returning `None`.

```rust
pub trait Block {
    fn pin(&self, idx: usize) -> Option<&Pin> { ... }
    fn pins(&self) -> &[Pin];
    fn constraints(&self) -> Vec<Constraint> { vec![] }
    fn kicad_symbol(&self) -> Option<&str> { None }   // NEW
}
```

**Rationale:** This lets `Design::add_component` copy the symbol reference into `ComponentRecord` alongside pins and constraints, without any new trait or indirection. The proc macro generates the override; manual impls can ignore it (they get `None` by default).

## Risks / Trade-offs

- **[Breaking change to `Pin`] Adding `pos` and `rotation` fields breaks every `Pin { ... }` struct literal.** → Mitigate by adding `#[derive(Default)]` to `Pin` (or a `Pin::builder()`) and updating literals. The number of call sites is small (~15 in `parts`, ~10 in tests).

- **[S-expression parser complexity]** Writing a parser for `.kicad_sym` files is new code. → The grammar is simple (lists, atoms, strings, line comments starting with `#`). We implement a minimal recursive-descent parser — not a full Lisp reader. ~150 LOC.

- **[Pin name matching ambiguity]** KiCad symbols may have pins named differently than the IR (e.g., numbered `"1"`/`"2"` vs named `"VDD"`/`"GND"`). → Mitigate by supporting match-by-number (add optional `number` field to `Pin` or match via the `#[pin(number = "1")]` attribute). Fall back to algorithmic spacing with a diagnostic if no match is found.

- **[Library file path]** `resolve_symbols` needs a path to the `.kicad_sym` file. → This is a CLI/build-time argument, not embedded in the IR. The CLI gains a `--symbol-lib <path>` flag. The IR stays tool-agnostic.

- **[Dependence on `syn`/`quote`/`proc-macro2`]** New workspace dependencies for the proc-macro crate. → These are standard, widely-used, and only affect the derive crate's build. The rest of the workspace is unaffected.

- **[Edition 2024]** The workspace uses `edition = "2024"`. Proc macros work fine with edition 2024 but `syn`/`quote` need to support it. → These crates are edition-agnostic; no issues expected.