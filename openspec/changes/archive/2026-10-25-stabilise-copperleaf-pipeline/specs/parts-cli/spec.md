# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
