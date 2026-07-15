# parts-cli Specification

## Purpose
Provide CLI commands for creating and updating part TOML manifests from KiCad symbol, footprint, and datasheet sources.

## Requirements

### Requirement: CLI exposes new and update commands
The `copperleaf-cli` binary SHALL provide two commands: `new` (create a part TOML from a source) and `update` (merge source data into an existing part TOML). Each command SHALL accept exactly one source selector: `--symbol <FILE> --lib-id <ID>`, `--footprint <FILE> --lib-id <ID>`, or `--datasheet <FILE>`. The binary SHALL use `clap` with `default-features = false`.

#### Scenario: new --symbol creates a part from a KiCad symbol
- **WHEN** `copperleaf new --symbol ic.kicad_sym --lib-id RP2354A --out rp2354a.toml` is run
- **THEN** a TOML file is created at `rp2354a.toml` containing pin names, kinds, and physical fields extracted from the symbol

#### Scenario: new --footprint creates a part from a KiCad footprint
- **WHEN** `copperleaf new --footprint ic.kicad_mod --lib-id RP2354A --out rp2354a.toml` is run
- **THEN** a TOML file is created at `rp2354a.toml` containing pad numbers, synthesised placeholder names, and physical fields extracted from the footprint

#### Scenario: new --datasheet hard-fails
- **WHEN** `copperleaf new --datasheet ic.pdf --out rp2354a.toml` is run
- **THEN** the CLI prints a `CLI:DATASHEET_STUB` diagnostic with severity Error
- **AND** exits with code 1
- **AND** no output file is written

#### Scenario: update --symbol merges symbol data into existing TOML
- **WHEN** `copperleaf update rp2354a.toml --symbol ic.kicad_sym --lib-id RP2354A` is run
- **THEN** the TOML file is updated with names and kinds for pins missing them
- **AND** manually-authored voltages, bandwidths, `nc`, and notes are preserved

#### Scenario: update --footprint merges physical data into existing TOML
- **WHEN** `copperleaf update rp2354a.toml --footprint ic.kicad_mod --lib-id RP2354A` is run
- **THEN** the TOML file is updated with `pos`, `rotation`, and `length` for pins matched by pad number
- **AND** all logical fields (names, kinds, voltages, bandwidths, notes) are left untouched

#### Scenario: update --datasheet hard-fails
- **WHEN** `copperleaf update rp2354a.toml --datasheet ic.pdf` is run
- **THEN** the CLI prints a `CLI:DATASHEET_STUB` diagnostic with severity Error
- **AND** exits with code 1

### Requirement: Symbol source extracts pins via sym_parser
The `new --symbol` and `update --symbol` flows SHALL use `copperleaf_backend_kicad::sym_parser::parse_symbol_lib` to parse the `.kicad_sym` file, `find_symbol` to select the symbol by `--lib-id`, and `flatten_extends` to resolve inherited pins. Each extracted `PinDef` SHALL be mapped to a Copperleaf `kind` via the kind-map.

#### Scenario: Symbol with extends resolves inherited pins
- **WHEN** a symbol `RP2354A` extends `RP2354_base` and `RP2354_base` defines pins `VDD` and `GND`
- **THEN** the generated TOML includes pins inherited from `RP2354_base`

#### Scenario: Symbol not found produces an error
- **WHEN** `--lib-id` does not match any symbol in the `.kicad_sym` file
- **THEN** the CLI prints a `CLI:SYMBOL_NOT_FOUND` diagnostic and exits with code 1

### Requirement: Footprint source extracts pads via fp_parser
The `new --footprint` and `update --footprint` flows SHALL use `copperleaf_backend_kicad::fp_parser::parse_footprint` to parse the `.kicad_mod` file. Each extracted `PadDef` SHALL contribute `pos`, `rotation`, and `length` to the TOML, keyed by pad number.

#### Scenario: Footprint-only part has placeholder names
- **WHEN** `new --footprint` is run without a prior symbol
- **THEN** pin names are synthesised as `PAD_<n>` where `<n>` is the pad number
- **AND** `kind` is set to `--default-kind` (default `dio`)

#### Scenario: Unmatched pad produces a warning
- **WHEN** `update --footprint` encounters a pad number not present in the existing TOML
- **THEN** a `CLI:UNMATCHED_PAD` warning is printed

### Requirement: Kind-map maps KiCad pin types to Copperleaf kinds
The CLI SHALL apply a built-in heuristic mapping from KiCad `pin_type` strings to Copperleaf `kind` values. Power pins (`power_in`, `power_out`) SHALL be mapped to `pwr` / `pwr_fixed` with `# TODO` placeholder comments for voltages. Unrecognised pin types SHALL fall back to `--default-kind` and emit a `CLI:UNKNOWN_PIN_TYPE` warning. A `--kind-map <FILE>` option SHALL load a TOML file with `[by_type]` and `[by_name]` tables that override the built-in map, with `[by_name]` taking precedence.

#### Scenario: power_in maps to pwr with TODO placeholder
- **WHEN** a symbol pin has `pin_type = "power_in"`
- **THEN** the TOML entry has `kind = "pwr"`
- **AND** a `# TODO: fill v_min/v_max/i_max` comment is emitted

#### Scenario: by_name override takes precedence over by_type
- **WHEN** `--kind-map file.toml` contains `[by_name]` entry for pin `"1V2O"` with `kind = "pwr_fixed"` and `[by_type]` entry for `"power_out"` with `kind = "pwr"`
- **AND** pin `1V2O` has `pin_type = "power_out"`
- **THEN** the pin is assigned `kind = "pwr_fixed"` from the `by_name` entry

#### Scenario: Unknown pin type warns and falls back
- **WHEN** a symbol pin has `pin_type = "free"` and no override is provided
- **THEN** the pin is assigned `--default-kind` (default `dio`)
- **AND** a `CLI:UNKNOWN_PIN_TYPE` warning is printed

### Requirement: Update merges by pin number preserving manual overrides
The `update` command SHALL merge source data into an existing TOML by matching on pin number. `--symbol` SHALL fill `kind` and kind-args only for pins where `kind` is absent or matches the `--default-kind` placeholder; it SHALL NOT overwrite voltages, bandwidths, `nc`, or `notes`. `--footprint` SHALL set/overwrite only `pos`, `rotation`, and `length`. Pins present in the source but not the existing TOML SHALL be appended with a `CLI:NEW_PIN` warning.

#### Scenario: Manual voltage preserved during symbol update
- **WHEN** an existing TOML pin has `kind = "pwr"` with `v_min = 2.97` and `v_max = 3.63`
- **AND** `update --symbol` is run on the same symbol
- **THEN** the `v_min` and `v_max` values are unchanged

#### Scenario: Placeholder name replaced during symbol update
- **WHEN** an existing TOML pin has `name = "PAD_4"` (a footprint placeholder)
- **AND** `update --symbol` finds a pin with `number = 4` and `name = "AVDD"`
- **THEN** the pin name is updated to `AVDD`

#### Scenario: New pin from symbol appended with warning
- **WHEN** the symbol contains a pin number not present in the existing TOML
- **THEN** the pin is appended to the TOML
- **AND** a `CLI:NEW_PIN` warning is printed

### Requirement: Vendor crate scaffolding via --crate flag
The `new` command SHALL accept a `--crate <VENDOR>` flag that creates a `parts/<vendor>/` directory with `Cargo.toml` (package `copperleaf-parts-<vendor>`, `[lib] path = "lib.rs"`, workspace deps) and `lib.rs` (module doc + `use copperleaf_part_macro::build_component;`), and appends `"parts/<vendor>"` to the root `Cargo.toml` `[workspace].members`.

#### Scenario: --crate creates vendor parts crate
- **WHEN** `copperleaf new --symbol ic.kicad_sym --lib-id W5500 --crate wiznet --out w5500.toml` is run
- **THEN** `parts/wiznet/Cargo.toml` exists with package name `copperleaf-parts-wiznet`
- **AND** `parts/wiznet/lib.rs` exists with `build_component` import
- **AND** the root `Cargo.toml` `[workspace].members` includes `"parts/wiznet"`
- **AND** `w5500.toml` is written to `parts/wiznet/w5500.toml`

### Requirement: CLI uses Diagnostic for output and conventional exit codes
All user-facing output SHALL use `copperleaf::Diagnostic { code, severity, message, entities, hint }` with `Severity` variants and `NAMESPACE:RULE` codes. The CLI SHALL introduce the `CLI:` namespace. Exit code SHALL be 0 on success and 1 on any error. Warnings SHALL NOT cause non-zero exit unless a future `--strict` flag is added.

#### Scenario: Successful new exits 0
- **WHEN** `copperleaf new --symbol ic.kicad_sym --lib-id RP2354A --out rp.toml` succeeds
- **THEN** the exit code is 0

#### Scenario: Error exits 1
- **WHEN** an error occurs (e.g. file not found, symbol not found)
- **THEN** a `Diagnostic` with `Severity::Error` is printed and the exit code is 1

### Requirement: part-codegen exposes TOML manifest schema
`part-codegen` SHALL make `Manifest`, `PinDef`, and `ComponentMeta` `pub` with both `Serialize` and `Deserialize` derives. `PinDef` SHALL include optional fields `pos: Option<(f64,f64)>`, `rotation: Option<f64>`, `length: Option<f64>`, and `nc: Option<bool>`. `part-codegen` SHALL expose a `validate(manifest) -> Vec<Diagnostic>` function that checks schema validity, duplicate pin names, unresolved power pins, and pin-name-to-const sanity.

#### Scenario: CLI serialises a Manifest to TOML
- **WHEN** the CLI builds a `Manifest` from symbol data and serialises it
- **THEN** the output TOML is deserialisable by `part-codegen::Manifest` without errors

#### Scenario: validate flags unresolved power pins
- **WHEN** a `Manifest` has a pin with `kind = "pwr"` and no `v_min`/`v_max`/`i_max`
- **THEN** `validate()` returns a `VALIDATE:UNRESOLVED_POWER` diagnostic

### Requirement: builder_expr emits physical fields
`part-codegen::builder_expr` SHALL append `.pos(x, y).rotation(r).length(l)` to the generated `PinBuilder` expression when `PinDef` carries `pos`, `rotation`, or `length` fields.

#### Scenario: Pin with physical data round-trips through codegen
- **WHEN** a TOML pin has `pos = [101.6, 12.7]`, `rotation = 90.0`, `length = 2.54`
- **THEN** the generated Rust expression contains `.pos(101.6, 12.7).rotation(90.0).length(2.54)`
