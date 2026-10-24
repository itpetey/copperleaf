# parts-cli Specification

## Purpose
Provide CLI commands for creating and updating part TOML manifests from KiCad symbol, footprint, and datasheet sources.

## Requirements

### Requirement: CLI exposes new and update commands
The `copperleaf-cli` binary SHALL provide `new` and `update` commands, each accepting exactly one source selector: `--symbol <FILE>`, `--footprint <FILE>`, `--dir <DIR>`, or `--datasheet <FILE>`. The binary SHALL use `clap` with `default-features = false`. `update` SHALL use `--crate <VENDOR>` and `--lib-id <ID>` to locate `parts/<vendor>/<toml_stem>.toml`, standardising with `new --crate`. Datasheet-derived output SHALL pass the same untrusted-manifest validation as every other source and SHALL NOT be compiled or executed automatically.

#### Scenario: new --symbol creates a part from a KiCad symbol
- **WHEN** `copperleaf new --symbol ic.kicad_sym --lib-id RP2354A --out rp2354a.toml` is run
- **THEN** a TOML file is created at `rp2354a.toml` containing pin names, kinds, and physical fields extracted from the symbol

#### Scenario: new --footprint creates a part from a KiCad footprint
- **WHEN** `copperleaf new --footprint ic.kicad_mod --lib-id RP2354A --out rp2354a.toml` is run
- **THEN** a TOML file is created at `rp2354a.toml` containing pad numbers, placeholder names, and physical fields extracted from the footprint

#### Scenario: new --dir discovers symbol and footprint from a directory
- **WHEN** `copperleaf new --dir ./part_files/ --lib-id RP2354A --crate raspberrypi` is run
- **AND** `./part_files/` contains `rp2354a.kicad_sym` and `rp2354a.kicad_mod`
- **THEN** a TOML file is created at `parts/raspberrypi/rp2354a.toml` containing both symbol-derived names/kinds and footprint-derived geometry

#### Scenario: new --datasheet produces a validated draft
- **WHEN** `copperleaf new --datasheet ic.pdf --out rp2354a.toml` is run with a usable datasheet
- **THEN** the CLI writes a manifest only after untrusted-manifest validation succeeds
- **AND** reports that the manifest is a draft requiring review
- **AND** does not build the generated Rust

#### Scenario: new --datasheet hard-fails
- **WHEN** datasheet processing returns content that fails manifest validation
- **THEN** the CLI prints an error diagnostic and exits with code 1
- **AND** no output file is written

#### Scenario: update --symbol merges symbol data into existing TOML
- **WHEN** `copperleaf update --crate raspberrypi --lib-id RP2354A --symbol ic.kicad_sym` is run
- **THEN** the TOML is updated with names and kinds for pins missing them
- **AND** manually authored voltages, bandwidths, `nc`, and notes are preserved

#### Scenario: update --footprint merges physical data into existing TOML
- **WHEN** `copperleaf update --crate raspberrypi --lib-id RP2354A --footprint ic.kicad_mod` is run
- **THEN** the TOML is updated with `pos`, `rotation`, and `length` for pins matched by pad number
- **AND** logical fields are left untouched

#### Scenario: update --dir discovers and merges both sources
- **WHEN** `copperleaf update --crate raspberrypi --lib-id RP2354A --dir ./part_files/` is run
- **AND** the directory contains a `.kicad_sym` and a `.kicad_mod` file
- **THEN** both symbol and footprint data are merged into the existing TOML

#### Scenario: update --datasheet hard-fails
- **WHEN** datasheet processing returns an invalid manifest update
- **THEN** the CLI prints an error diagnostic and exits with code 1
- **AND** the existing TOML remains unchanged

#### Scenario: update --datasheet preserves reviewed data
- **WHEN** `copperleaf update --crate raspberrypi --lib-id RP2354A --datasheet ic.pdf` is run
- **THEN** the CLI proposes a validated manifest update
- **AND** preserves the existing base64 model payload
- **AND** does not overwrite an existing TOML until validation succeeds

### Requirement: update locates a part by crate and lib id
`update` SHALL use `--crate <VENDOR>` and `--lib-id <ID>` to locate the part TOML at `parts/<vendor>/<toml_stem>.toml`, standardising with `new --crate`.

#### Scenario: update derives the part path from crate and lib id
- **WHEN** `copperleaf update --crate wiznet --lib-id W5500` is run
- **THEN** the existing manifest at `parts/wiznet/w5500.toml` is located for update

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

### Requirement: --dir discovers symbol and footprint files in a directory
When `--dir <DIR>` is specified, the CLI SHALL scan the directory for `.kicad_sym` and `.kicad_mod` files (or a `.pretty` footprint library directory). If a symbol file is found it SHALL be processed first, seeding pin names and kinds. If a footprint file is found its pads SHALL be merged into the same manifest, providing geometry. If neither file type is found the CLI SHALL error with `CLI:NO_SOURCE`.

#### Scenario: --dir with both files processes both
- **WHEN** `new --dir ./src/ --lib-id TEST --crate myvendor` is run
- **AND** `./src/` contains `test.kicad_sym` and `test.kicad_mod`
- **THEN** the output TOML contains pin names/kinds from the symbol AND geometry from the footprint

#### Scenario: --dir with only symbol processes symbol
- **WHEN** `new --dir ./src/ --lib-id TEST --crate myvendor` is run
- **AND** `./src/` contains only a `.kicad_sym` file
- **THEN** the output TOML contains pin names and kinds from the symbol

#### Scenario: --dir with only footprint processes footprint
- **WHEN** `new --dir ./src/ --lib-id TEST --crate myvendor` is run
- **AND** `./src/` contains only a `.kicad_mod` file
- **THEN** the output TOML contains synthesised pin names and pad geometry

#### Scenario: --dir with no KiCad files errors
- **WHEN** `new --dir ./empty/` is run and `./empty/` contains no `.kicad_sym` or `.kicad_mod` files
- **THEN** the CLI prints a `CLI:NO_SOURCE` diagnostic and exits with code 1

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
Both `new` and `update` SHALL accept a `--crate <VENDOR>` flag. For `new`, it controls output location at `parts/<vendor>/<toml_stem>.toml` and scaffolds a vendor parts crate. For `update`, `--crate` and `--lib-id` together locate the existing part TOML.

The scaffolding SHALL create `parts/<vendor>/` with a `Cargo.toml` (package `copperleaf-parts-<vendor>`, `[lib] path = "lib.rs"`, workspace deps) and `lib.rs` (module doc plus `use copperleaf_part_macro::build_component;`).

#### Scenario: --crate creates vendor parts crate
- **WHEN** `copperleaf new --symbol ic.kicad_sym --lib-id W5500 --crate wiznet` is run
- **THEN** `parts/wiznet/Cargo.toml` exists with package name `copperleaf-parts-wiznet`
- **AND** `parts/wiznet/lib.rs` exists with `build_component` import
- **AND** `w5500.toml` is written to `parts/wiznet/w5500.toml`

#### Scenario: update --crate locates the part TOML
- **WHEN** `copperleaf update --crate wiznet --lib-id W5500 --footprint w5500.kicad_mod` is run
- **THEN** the TOML at `parts/wiznet/w5500.toml` is read and updated

### Requirement: Workspace membership updates are TOML-aware
Scaffolding SHALL append `"parts/<vendor>"` to the root `Cargo.toml` `[workspace].members` list, preserving valid TOML whether the list was empty or populated.

#### Scenario: Existing workspace gains the new member
- **WHEN** a vendor crate is scaffolded in a workspace with populated `members`
- **THEN** the root `Cargo.toml` `[workspace].members` includes `"parts/<vendor>"`
- **AND** the resulting manifest parses successfully

#### Scenario: update with non-existent part errors
- **WHEN** `copperleaf update --crate wiznet --lib-id NOPART` is run and no TOML exists
- **THEN** the CLI prints a `CLI:PART_NOT_FOUND` diagnostic and exits with code 1

### Requirement: CLI uses Diagnostic for output and conventional exit codes
All user-facing output SHALL use `copperleaf::Diagnostic { code, severity, message, entities, hint }` with `Severity` variants and `NAMESPACE:RULE` codes. The CLI SHALL introduce the `CLI:` namespace. Exit code SHALL be 0 on success and 1 on any error. Warnings SHALL NOT cause non-zero exit unless a future `--strict` flag is added.

#### Scenario: Successful new exits 0
- **WHEN** `copperleaf new --symbol ic.kicad_sym --lib-id RP2354A --out rp.toml` succeeds
- **THEN** the exit code is 0

#### Scenario: Error exits 1
- **WHEN** an error occurs (e.g. file not found, symbol not found)
- **THEN** a `Diagnostic` with `Severity::Error` is printed and the exit code is 1

### Requirement: part-codegen exposes TOML manifest schema
`part-codegen` SHALL make `Manifest`, `PinDef`, and `ComponentMeta` public with `Serialize` and `Deserialize` derives. `PinDef` SHALL include optional physical fields `pos`, `rotation`, `length`, and `nc`. Manifest validation SHALL reject duplicate physical pad numbers, not duplicate human-readable names, and SHALL check schema validity, unresolved power pins, and pin-name-to-constant sanity.

#### Scenario: CLI serialises a Manifest to TOML
- **WHEN** the CLI builds a `Manifest` from symbol data and serialises it
- **THEN** the output TOML is deserialisable by `part-codegen::Manifest` without errors

#### Scenario: validate flags unresolved power pins
- **WHEN** a `Manifest` has a pin with `kind = "pwr"` and no `v_min`/`v_max`/`i_max`
- **THEN** validation returns a `VALIDATE:UNRESOLVED_POWER` diagnostic

#### Scenario: Repeated names with unique pads are valid
- **WHEN** two pins are named `AVDD` and declare pad numbers `4` and `8`
- **THEN** validation accepts both pins

#### Scenario: Duplicate pad numbers are invalid
- **WHEN** two pins declare pad number `7`
- **THEN** validation returns a duplicate-pad diagnostic

### Requirement: Generated Rust treats manifests as untrusted input
Every manifest generation path SHALL parse quantities into a closed typed grammar rather than accepting Rust expressions, escape every generated string as a Rust literal, validate generated identifiers, and reject unknown schema fields before writing output. Invalid input SHALL produce diagnostics without partial code generation.

#### Scenario: String cannot escape generated Rust
- **WHEN** a pin name or metadata value contains quotes, newlines, comment markers, or Rust syntax
- **THEN** generated code contains an escaped literal and cannot execute the injected text

#### Scenario: Rust expression is rejected as a quantity
- **WHEN** a quantity field contains `std::process::exit(1)`
- **THEN** validation rejects the value as an invalid quantity
- **AND** no generated source file is written

#### Scenario: Invalid struct name is rejected
- **WHEN** a manifest component name is not a valid Rust identifier
- **THEN** generation reports an invalid-identifier diagnostic
- **AND** writes no source file

#### Scenario: Unknown field is rejected
- **WHEN** a manifest contains an unrecognised schema field
- **THEN** deserialisation reports the field rather than silently ignoring it

### Requirement: Manifest serialisation is lossless
Serialising and deserialising a manifest SHALL preserve every semantic field, including component metadata, physical pins, mechanical pads, electrical constraints, and the `[layout]` table. An update SHALL NOT drop layout directives or replace reviewed values with defaults.

#### Scenario: Layout survives update round-trip
- **WHEN** an existing manifest contains net-class and creepage layout directives
- **AND** it is serialised for an update and read back
- **THEN** every `[layout]` value remains present and unchanged

#### Scenario: Semantic round-trip is complete
- **WHEN** a manifest is deserialised, serialised, and deserialised again
- **THEN** both manifest values contain identical semantic content

### Requirement: Generated paths remain within the intended root
The CLI SHALL reject output names and manifest-derived path components that are absolute, contain parent traversal, or resolve outside the intended workspace or output directory after symlink resolution. Generated paths SHALL be checked before any file is written.

#### Scenario: Parent traversal is rejected
- **WHEN** a project name, footprint name, or output path resolves outside the requested root
- **THEN** the CLI reports a path-containment error
- **AND** writes no file outside that root

#### Scenario: Absolute manifest filename is rejected
- **WHEN** a manifest supplies an absolute footprint or model filename for generated output
- **THEN** generation rejects the filename and names the offending manifest field

### Requirement: Vendor scaffolding produces a rebuild-safe crate
Vendor scaffolding SHALL emit a package-local rebuild hook that tracks part TOML inputs, SHALL produce valid workspace membership when the members list is empty or populated, and SHALL fail rather than report success when workspace metadata cannot be updated.

#### Scenario: New vendor crate tracks TOML changes
- **WHEN** a vendor crate is scaffolded
- **THEN** the crate contains a build script that emits rebuild tracking for its TOML files

#### Scenario: Empty members list remains valid TOML
- **WHEN** the root workspace has `members = []`
- **AND** a vendor crate is added
- **THEN** the resulting workspace manifest parses successfully
- **AND** contains the new member

### Requirement: part-codegen validates manifest semantics
`part-codegen` SHALL expose a `validate(manifest) -> Vec<Diagnostic>` function that checks schema validity, duplicate physical pad numbers, unresolved power pins, and pin-name-to-const sanity.

#### Scenario: validate flags unresolved power pins
- **WHEN** a `Manifest` has a pin with `kind = "pwr"` and no `v_min`/`v_max`/`i_max`
- **THEN** `validate()` returns a `VALIDATE:UNRESOLVED_POWER` diagnostic

### Requirement: builder_expr emits physical fields
`part-codegen::builder_expr` SHALL append `.pos(x, y).rotation(r).length(l)` to the generated `PinBuilder` expression when `PinDef` carries `pos`, `rotation`, or `length` fields.

#### Scenario: Pin with physical data round-trips through codegen
- **WHEN** a TOML pin has `pos = [101.6, 12.7]`, `rotation = 90.0`, `length = 2.54`
- **THEN** the generated Rust expression contains `.pos(101.6, 12.7).rotation(90.0).length(2.54)`
