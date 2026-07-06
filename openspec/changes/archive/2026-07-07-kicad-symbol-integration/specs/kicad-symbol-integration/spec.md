## ADDED Requirements

### Requirement: IR carries KiCad symbol reference on components
The `ComponentRecord` struct SHALL include a `kicad_symbol: Option<String>` field. The `Block` trait SHALL include `fn kicad_symbol(&self) -> Option<&str>` with a default implementation returning `None`. `Design::add_component` SHALL copy the result of `kicad_symbol()` into the `ComponentRecord`.

#### Scenario: Component without symbol reference
- **WHEN** a `Block` impl does not override `kicad_symbol()`
- **THEN** the resulting `ComponentRecord` has `kicad_symbol == None`

#### Scenario: Component with symbol reference
- **WHEN** a `Block` impl returns `Some("RP2040:RP2354a")` from `kicad_symbol()`
- **THEN** the resulting `ComponentRecord` has `kicad_symbol == Some("RP2040:RP2354a")`

### Requirement: IR carries per-pin position metadata
The `Pin` struct SHALL include `pos: Option<(f64, f64)>` and `rotation: Option<f64>` fields representing the pin's position in mm relative to the symbol origin and its rotation in degrees. `Pin::new()` SHALL initialize both to `None`. `Pin` SHALL derive `Default`.

#### Scenario: Pin without position
- **WHEN** a pin is created via `Pin::new("VDD", Role::PowerIn, limits, None)`
- **THEN** `pin.pos == None` and `pin.rotation == None`

#### Scenario: Pin with explicit position
- **WHEN** a pin's `pos` is set to `Some((-10.16, 5.08))` and `rotation` to `Some(0.0)`
- **THEN** the schematic emitter places the pin at that coordinate within the symbol body

### Requirement: S-expression parser reads KiCad symbol library files
The KiCad backend SHALL provide a parser for `.kicad_sym` files that extracts symbol names, pin names, pin numbers, pin positions `(x, y)`, pin rotations, pin types, and pin lengths. The parser SHALL return a list of `SymbolDef` structs.

#### Scenario: Parse a symbol library file
- **WHEN** a `.kicad_sym` file containing `(symbol "RP2354a" ... (pin power_in line (at -15.24 5.08 0) (length 2.54) (name "VDD" ...) (number "1" ...)) ...)` is parsed
- **THEN** the resulting `SymbolDef` has `lib_id == "RP2354a"` and a `PinDef` with `name == "VDD"`, `number == "1"`, `pos == (-15.24, 5.08)`, `rotation == 0.0`, `pin_type == "power_in"`, `length == 2.54`

#### Scenario: Parse a library with multiple symbols
- **WHEN** a `.kicad_sym` file contains two `(symbol ...)` entries named "RP2354a" and "RP2040"
- **THEN** the parser returns two `SymbolDef` entries

### Requirement: resolve_symbols populates pin positions from symbol libraries
The KiCad backend SHALL provide `fn resolve_symbols(design: &mut Design, lib_path: &str)` that, for each component where `kicad_symbol` is set and pins lack positions, parses the `.kicad_sym` file at `lib_path`, finds the matching symbol, and sets each pin's `pos` and `rotation` by matching pin names.

#### Scenario: Resolve pins for a known symbol
- **WHEN** `resolve_symbols` is called on a design where `U1` has `kicad_symbol == Some("RP2040:RP2354a")` and pins named "VDD", "GND"
- **AND** the symbol library file contains a symbol "RP2354a" with pins named "VDD" at `(-15.24, 5.08, 0)` and "GND" at `(-15.24, -5.08, 0)`
- **THEN** after resolution, `U1`'s "VDD" pin has `pos == Some((-15.24, 5.08))` and `rotation == Some(0.0)`, and "GND" has `pos == Some((-15.24, -5.08))` and `rotation == Some(0.0)`

#### Scenario: Unresolved symbol emits diagnostic and falls back
- **WHEN** `resolve_symbols` is called but the symbol `"RP2040:RP2354a"` is not found in the library file
- **THEN** a `Diagnostic` with severity `Warning` is added to the design
- **AND** the affected pins retain `pos == None` (algorithmic fallback in the emitter)

#### Scenario: Unresolved pin by name emits diagnostic
- **WHEN** the symbol is found but a pin named "VDD" in the IR does not match any pin in the symbol library
- **THEN** a `Diagnostic` with severity `Warning` is added naming the unresolved pin
- **AND** that pin retains `pos == None`

### Requirement: Schematic emitter uses real symbol references and pin positions
The schematic emitter SHALL use `kicad_symbol` for the `lib_id` of symbol instances when present, falling back to `copperleaf:{refdes}` when absent. When a pin has `pos` and `rotation`, the emitter SHALL place the pin at those coordinates within the lib_symbol definition and compute wire/label endpoints from those coordinates instead of the algorithmic `pin_y_offset`.

#### Scenario: Symbol with kicad_symbol and resolved pins
- **WHEN** `emit_schematic` is called on a design where `U1` has `kicad_symbol == Some("RP2040:RP2354a")` and pins with `pos` set
- **THEN** the symbol instance uses `(lib_id "RP2040:RP2354a")`
- **AND** the lib_symbol pin `(at x y rotation)` matches the pin's `pos` and `rotation`
- **AND** wire endpoints coincide with the pin tip positions (pos adjusted by pin length and rotation)

#### Scenario: Symbol without kicad_symbol falls back to generic
- **WHEN** `emit_schematic` is called on a design where `U1` has `kicad_symbol == None` and pins with `pos == None`
- **THEN** the symbol instance uses `(lib_id "copperleaf:U1")`
- **AND** pins are placed algorithmically via `pin_y_offset` (existing behavior)

### Requirement: derive(Component) proc macro generates Block impl
The `copperleaf-derive` crate SHALL provide `#[derive(Component)]` which generates `impl Block` for the annotated struct. The struct SHALL have a `pins: Vec<Pin>` field. A `#[component(symbol = "lib_id")]` attribute SHALL override the generated `kicad_symbol()` method.

#### Scenario: Derive on a simple struct
- **WHEN** `#[derive(Component)]` is applied to a struct with a `pins: Vec<Pin>` field
- **THEN** the generated `impl Block` has `pins()` returning `&self.pins`, `constraints()` returning `vec![]`, and `kicad_symbol()` returning `None`

#### Scenario: Derive with symbol attribute
- **WHEN** `#[derive(Component)]` and `#[component(symbol = "RP2040:RP2354a")]` are applied to a struct
- **THEN** the generated `kicad_symbol()` returns `Some("RP2040:RP2354a")`

#### Scenario: Derive re-exported through edsl
- **WHEN** `copperleaf-edsl` re-exports `Component` from `copperleaf-derive`
- **AND** a downstream crate uses `use copperleaf_edsl::Component;`
- **THEN** `#[derive(Component)]` is available as if it came from `copperleaf-edsl`