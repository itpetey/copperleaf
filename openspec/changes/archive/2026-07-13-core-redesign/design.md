## Context

The `main/` codebase proved the Copperleaf concept: typed units, constraint-driven ERC, decoupling synthesis, KiCad schematic/PCB emission. But its workflow is inverted — the user authors in Rust, serialises to JSON, then runs a CLI to import into a backend. The `main.rs` should be the source of truth; `cargo run` should be the entire workflow.

The `new/` codebase sketches a cleaner architecture (`Board` → `compile()` → `CompiledBoard`, `Pin::build()` ergonomics, `Component` trait) but is incomplete: no nets, no constraints, no serialisation, no ERC, no backends, and several bugs (`ComponentPin::From<&str>` iterator consumption, random UUIDs breaking determinism, `decouple: bool` with no accessor).

This change merges the two: take `new/`'s architectural shape, fill it with `main/`'s proven substance. The result is a code-first EDA library where `main.rs` is the source of truth, components are self-contained (no filesystem dependencies), and `board.compile()` runs ERC + synthesis before handing a `CompiledBoard` to a backend.

Key constraints from discussion:
- Components are code-only — no external symbol/footprint file dependencies. Physical data is embedded as `&'static str`.
- Pin references are strictly typed — no string fallback. `PinRef` constants on components, `ComponentHandle`/`PinHandle` from `board.add()`/`.pin()`.
- `PowerSpec` carries `v_nom` for voltage inference. Fixed pins set `v_nom = v_min = v_max`; flexible pins leave it `None`.
- Pin physical data (`pos`, `rotation`, `length`) is extracted during component generation (by a future generator CLI), not at board compile time. A faulty S-expression fails during component authoring, not end-user compilation.
- International English throughout (e.g. "synthesised", "colour").

## Goals / Non-Goals

**Goals:**
- Establish the `Board` → `compile()` → `CompileReport`/`CompileError` → `Backend::emit()` workflow as the primary (and only) workflow.
- Make `main.rs` the single source of truth for a board project — no JSON intermediary, no CLI entry point.
- Support code-only components distributable via `crates.io` with embedded physical data.
- Type-safe pin references with no string fallback.
- Bring across from `main/`: `Net`, `Constraint`, `SigSpec`, `NetKind`, `NetClass`, ERC rules, `synthesize_decoupling`, KiCad emitters, S-expression parser, `sym_parser`, deterministic IDs.
- Net inference from connectivity with explicit override.
- `CompileReport` with inspectable synthesis results so the developer can audit compiler decisions.

**Non-Goals:**
- The component generator CLI (`copperleaf-gen`) — separate changeset. This changeset defines the `Component` trait shape that the generator will target.
- `#[derive(Component)]` proc macro — separate changeset. The trait shape supports it.
- Layout/routing/DRC — out of scope. Copperleaf produces correct schematics + netlists + a rough PCB starting point. The developer lays out by hand.
- Full-wave EM simulation, SI/PI post-layout analysis, thermal analysis.
- Serialisation as a workflow intermediary. `CompiledBoard::to_json()` exists as a derived inspection view, not as the primary interface.
- Infinite part coverage. A small, high-quality stdlib plus user-defined parts.

## Decisions

### D1: `Board` as mutable builder, `CompiledBoard` as immutable artifact

**Decision:** `Board` is the mutable, in-progress design. `board.compile(self) -> Result<CompileReport, CompileError>` consumes it, runs ERC + synthesis + resolution, and returns a `CompileReport` containing the immutable `CompiledBoard`.

```rust
pub struct Board {
    components: Vec<ComponentEntry>,
    connections: Vec<Connection>,
    net_overrides: HashMap<NetId, NetOverride>,
}

pub struct CompiledBoard {
    components: Vec<CompiledComponent>,  // original + synthesised, with deterministic refdes
    nets: Vec<Net>,                      // inferred + overridden
    connections: Vec<Connection>,        // resolved
    constraints: Vec<Constraint>,        // explicit + inferred
}
```

**Rationale:** `compile()` consuming `self` prevents post-compile mutation. The developer must explicitly compile to get an artifact the backend accepts. This is the type-level enforcement of "compiled = frozen."

**Alternative considered:** `compile(&self)` with internal mutability — rejected because it allows mutating a board whose compiled artifact is already in use, creating aliasing bugs.

### D2: `ComponentHandle` / `PinHandle` / `PinRef` — strictly typed pin references

**Decision:** `board.add("rpi", Rp2354a::new())` returns a `ComponentHandle`. `handle.pin(Rp2354a::IOVDD)` returns a `PinHandle`. `board.connect()` accepts only `PinHandle`s. No `impl From<&str>`.

```rust
pub struct PinRef(pub &'static str);

pub struct ComponentHandle(usize);  // index into Board's component vec

impl ComponentHandle {
    pub fn pin(&self, pin: PinRef) -> PinHandle { ... }
}

pub struct PinHandle {
    component: usize,
    pin: &'static str,
}
```

Components expose pin constants:
```rust
impl Rp2354a {
    pub const IOVDD: PinRef = PinRef("IOVDD");
    pub const DVDD: PinRef = PinRef("DVDD");
    // ...
}
```

**Rationale:** Eliminates an entire class of runtime errors (typos in pin names, wrong component names). The constant is defined once; every reference is auditable. The blast radius of a typo is the definition site, not every call site.

**Alternative considered:** String fallback (`impl From<&str>`) — rejected by user decision. Strict typing only.

**On `PinRef` using `&'static str`:** The string inside `PinRef` is not used for runtime lookup in `connect()`. `PinHandle` carries the component index and pin name, and `connect()` validates the pin name exists on the component at compile time (board compile, not Rust compile). The `&'static str` ensures pin constants are compile-time literals with no allocation.

### D3: `PowerSpec` with `v_nom` for voltage inference

**Decision:** Replace `PowerLimit` with `PowerSpec`, adding `v_nom: Option<Qty<Volt>>`.

```rust
pub struct PowerSpec {
    pub v_min: Qty<Volt>,
    pub v_max: Qty<Volt>,
    pub v_nom: Option<Qty<Volt>>,
    pub i_max: Qty<Amp>,
}
```

Builder methods:
- `pwr_fixed(v, i)` — `v_nom = v_min = v_max = v`. For fixed-voltage pins (DVDD = 1.1V).
- `pwr(v_min, v_max, i)` — `v_nom = None`. For flexible pins (IOVDD = 1.8–3.3V).
- `.nominal(v)` — chainable override to set `v_nom` on a flexible pin.

**Net voltage inference rules (in compile()):**
1. Net has a pin with `v_nom = Some(V)` → net voltage = V. Multiple pins with disagreeing `v_nom` → ERC error.
2. Net has no `v_nom` pins → compiler checks for a connected `PowerOut` with known output voltage (future: regulator output). For now, no `PowerOut` voltage model exists, so this falls through.
3. Neither → "net has no voltage source" error, blocks compile. Developer declares via `NetHandle::set_voltage()`.

### D4: `Net` as inferred entity with explicit override

**Decision:** `connect()` creates pin-to-pin edges. During `compile()`, connected components in the graph become `Net`s with inferred properties. The developer can override via `NetHandle` returned from `connect()`.

```rust
// connect() returns a NetHandle for optional annotation
let net = board.connect(rpi.pin(Rp2354a::IOVDD), radio.pin(Mm8108::VBAT))?;
net.set_voltage(3.3.volt());
```

`NetHandle` is a lightweight reference to an emerging net identity. Multiple `connect()` calls that join the same connected component share a net. `Net` (on `CompiledBoard`) carries the final inferred + overridden properties.

**Rationale:** Inverts `main/`'s model (create nets first, then connect pins to them) to `new/`'s model (connect pins, nets emerge, annotate as needed). This is more natural for the target user — they think "connect these two pins," not "create a net, then attach pins."

**`NetHandle` identity:** Since `connect()` may join two existing sub-nets, the handle must resolve to the merged net at compile time. `NetHandle` stores a representative edge index; `compile()` uses union-find to resolve final net membership. The handle's `set_voltage()` records an override keyed by the representative, which is merged during net resolution.

### D5: Code-only components — embedded physical data

**Decision:** `Component` trait carries physical data as `&'static str`, not file paths.

```rust
pub trait Component {
    fn pins(&self) -> &[Pin];
    fn pin(&self, id: PinId) -> Option<&Pin>;
    fn pin_name(&self, name: &str) -> Option<&Pin>;
    fn constraints(&self) -> Vec<Constraint> { vec![] }
    fn symbol(&self) -> Option<&'static str> { None }
    fn footprint(&self) -> Option<&'static str> { None }
}
```

Pin physical fields (`pos`, `rotation`, `length`) are set at component construction time — by hand or by the generator CLI. `compile()` never parses S-expressions.

**Rationale:** Eliminates the "works on my machine" problem. `cargo add copperleaf-parts-rp2350` gives you everything. No `KICAD_SYMBOL_DIR`, no filesystem resolution, no path-dependent behaviour. Components are deterministic, version-pinned, self-contained units.

**Backend-agnostic naming:** Methods are `symbol()` / `footprint()`, not `kicad_symbol()` / `kicad_footprint()`. The payload format happens to be KiCad S-expressions today, but the trait doesn't bake the backend into its API. When a second backend arrives, the component grows new methods or a generic payload mechanism.

### D6: `CompileReport` and `CompileError`

**Decision:**
```rust
pub struct CompileReport {
    pub board: CompiledBoard,
    pub warnings: Vec<Diagnostic>,
    pub summary: CompileSummary,
}

pub struct CompileSummary {
    pub nets: Vec<NetInfo>,
    pub caps_synthesised: Vec<SynthCap>,
    pub pin_count: usize,
    pub component_count: usize,
}

pub struct CompileError {
    pub errors: Vec<Diagnostic>,
}
```

`board.compile()` returns `Result<CompileReport, CompileError>`. Errors block; warnings don't. The developer can inspect `CompileReport.summary` to audit what the compiler did (which nets were inferred, which caps were synthesised). `CompileError` carries *all* errors so the developer sees every problem in one pass.

### D7: Deterministic IDs via FNV-1a

**Decision:** Replace `Uuid::new_v4()` with deterministic IDs derived from FNV-1a hashes of stable seeds (component name + pin name, or refdes + pin name).

```rust
pub fn deterministic_id(seed: &str) -> String {
    // FNV-1a 64-bit hash, formatted as 8-4-4-4-12 hex
    // Same seed → same ID, always
}
```

**Rationale:** Deterministic output is required for diffs, snapshot tests, and AI round-trip workflows. Random UUIDs produce different output on every run, making any comparison meaningless. `main/` already implements this — bring it across directly.

`PinId` changes from `Uuid` to a newtype wrapping `String` (the deterministic hash). `Uuid` crate dependency is removed.

### D8: `Backend` trait

**Decision:**
```rust
pub trait Backend {
    type Error;
    fn emit(&self, output_dir: &str, board: &CompiledBoard) -> Result<(), Self::Error>;
}
```

KiCad backend implements this. Future backends (SPICE, IPC-2581) implement the same trait. The developer's `main.rs` constructs a backend and calls `emit()`:

```rust
let backend = KiCad::new();
backend.emit("path/to/output/", &report.board)?;
```

**Rationale:** Makes the backend pluggable without the core knowing about any specific backend. The `CompiledBoard` is backend-agnostic — it carries all resolved data, the backend translates it to its target format.

### D9: Serialisation as derived view

**Decision:** `CompiledBoard` implements `Serialize` (and `Deserialize` for tooling), but serialisation is not part of the primary workflow. It exists for inspection, diffing, and potential future GUI tools.

```rust
let report = board.compile()?;
let json = serde_json::to_string_pretty(&report.board)?;
// or: report.board.to_json() convenience method
```

**Rationale:** In `main/`, JSON was the workflow intermediary — the CLI loaded JSON, ran analysis, emitted to backend. With `main.rs` as SOT, the JSON intermediary is eliminated. But serialisation is still useful as a derived view: the AI loop can inspect `CompiledBoard` JSON, diff tools can compare boards, and a future GUI could consume it. Demoting it from "workflow" to "inspection aid" is the right level.

### D10: Crate structure

**Decision:** New workspace layout:
```
crates/
├── model/          → copperleaf-model (Board, CompiledBoard, Pin, Component, Net, types)
├── parts/          → copperleaf-parts (stdlib passives: Capacitor, Resistor, etc.)
├── analysis/       → copperleaf-analysis (ERC rules, synthesize_decoupling)
├── backend-kicad/  → copperleaf-backend-kicad (KiCad emitters, sexpr, sym_parser)
└── derive/         → copperleaf-derive (future — #[derive(Component)], not this changeset)
```

`analysis` and `backend-kicad` are new crates brought across from `main/`. `model` absorbs the net/constraint/sigspec types that were in `main/`'s `ir` crate — there's no separate `ir` crate in `new/` because the model *is* the IR.

**Alternative considered:** Separate `ir` crate as in `main/` — rejected. `main/` separated `core` (units/diagnostics) and `ir` (data model) because the CLI and backends needed to depend on `ir` without pulling in the EDSL. In `new/`, the `model` crate serves both roles — it contains units, diagnostics, and the data model. The crate count stays low and the dependency graph stays flat. If the model crate grows large, it can be split later.

## Risks / Trade-offs

- **[Strict typing requires constants on every component]** Every part must define `PinRef` constants for all pins. A 60-pin QFN has 60 constant lines. → Mitigated by the generator CLI emitting them automatically. For hand-written parts, it's tedious but auditable. The alternative (string fallback) was explicitly rejected.

- **[Net inference is approximate]** Voltage inference from `v_nom` works for fixed pins but fails for flexible pins without a connected regulator model. → Mitigated by explicit `NetHandle::set_voltage()` override. The compiler errors loudly when it can't infer, rather than guessing silently. This is the correct behaviour — a wrong guess is worse than a loud error.

- **[No `ir` crate separation]** `model` carries both units and data model, unlike `main/`'s split. → If `model` grows unwieldy, split into `model-core` (units, diagnostics) and `model` (data model) later. YAGNI for now.

- **[`NetHandle` identity across merges]** When `connect()` joins two existing sub-nets, the handle's override must merge correctly. → Union-find during `compile()` resolves final net membership. Overrides are merged by representative. This is well-understood algorithmically.

- **[Component trait method names imply backend format]** `symbol()` / `footprint()` return KiCad S-expressions today. → Names are backend-agnostic; payload format is an implementation detail. When a second backend arrives, either add new methods or introduce a generic payload mechanism. Don't over-engineer for a second backend that doesn't exist yet.

- **[`PinId` as `String` instead of `Uuid`]** Deterministic IDs are strings (FNV-1a hex), not UUIDs. → This is deliberate. `Uuid` implies randomness; deterministic strings enable reproducible builds. The `uuid` crate dependency is removed.

## Migration Plan

This is a fresh codebase (`new/`). No migration of existing users needed. Downstream projects (e.g. `halow-sta/new/`) update their `main.rs` and part definitions to match the new API.

The implementation order (see tasks.md) is bottom-up: model types → parts → analysis → backend → integration tests. Each layer is testable before the next is built.

## Open Questions

- **Regulator output voltage model:** `PowerOut` pins don't yet carry a nominal output voltage. For now, net inference relies on `PowerIn` pins' `v_nom` or explicit `NetHandle::set_voltage()`. Adding `PowerSpec` to `PowerOut` pins (with a nominal output) is a natural extension but deferred to a future changeset.
- **`NetHandle` API shape:** Whether `connect()` returns `Result<NetHandle, CompileError>` or `Result<(), CompileError>` with a separate `board.net()` lookup. The return-value approach is preferred (no string lookup needed) but requires the handle to be valid before `compile()` runs (pre-compile net identity is provisional).
- **Multiple boards per project:** The `Board` docstring says "1 or more." This changeset supports a single `Board` per `main.rs`. Multi-board projects (e.g. carrier + daughter card) are a future concern.
