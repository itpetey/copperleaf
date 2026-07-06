## ADDED Requirements

### Requirement: Part macro generates struct and Block impl
The `copperleaf-edsl` crate SHALL provide a `part!` declarative macro that generates a part struct, a `new()` constructor, and an `impl Block` from a pin table and optional constraint list.

#### Scenario: Define a simple part
- **WHEN** `part! { pub struct MyChip("MYCHIP"); pins: VDD = power_in(1.7.volt(), 3.6.volt(), 0.5.amp()), GND = gnd(), ; }` is invoked
- **THEN** a struct `MyChip` is generated with a `new(id: &str) -> Self` constructor
- **AND** `MyChip::new("MYCHIP").pins().len() == 2`

#### Scenario: Define a part with constraints
- **WHEN** `part!` is invoked with a `constraints:` section listing `Decoupling` and `MaxJunction` constraints
- **THEN** the generated `impl Block` includes `constraints()` returning those constraints

### Requirement: Part macro supports pin helper functions
The `part!` macro SHALL accept pin helper function calls in the pin table. Pin helpers are small functions returning `Pin` (e.g., `gnd()`, `dio()`, `power_in(v_min, v_max, i_max)`, `dio_spi(bw, z)`).

#### Scenario: Pin helpers generate correct pins
- **WHEN** a `part!` definition uses `gnd()` for a pin
- **THEN** the generated pin has `role == Role::Gnd`
- **WHEN** a `part!` definition uses `power_in(1.62.volt(), 3.6.volt(), 0.2.amp())` for a pin
- **THEN** the generated pin has `role == Role::PowerIn` and limits matching the arguments

### Requirement: Part macro supports duplicate pin names
The `part!` macro SHALL allow duplicate pin names in the pin table (e.g., multiple `GND` pads). Each entry SHALL generate a separate `Pin` in the resulting `Vec<Pin>`.

#### Scenario: Multiple GND pins
- **WHEN** a `part!` definition has three `GND = gnd(),` entries
- **THEN** the generated `pins()` vec contains three `Pin` entries all named "GND" with `Role::Gnd`
