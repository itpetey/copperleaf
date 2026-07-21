## MODIFIED Requirements

### Requirement: Footprint source extracts pads via fp_parser
The `new --footprint` and `update --footprint` flows SHALL use `copperleaf_backend_kicad::fp_parser::parse_footprint` to parse the `.kicad_mod` file. Each extracted `PadDef` SHALL be converted to a core `Pad` (see the `pad-model` spec) and SHALL contribute the pin's full pad geometry to the TOML, keyed by pad number.

#### Scenario: Footprint-only part has placeholder names
- **WHEN** `new --footprint` is run without a prior symbol
- **THEN** pin names are synthesised as `PAD_<n>` where `<n>` is the pad number
- **AND** `kind` is set to `--default-kind` (default `dio`)

#### Scenario: Unmatched pad produces a warning
- **WHEN** `update --footprint` encounters a pad number not present in the existing TOML
- **THEN** a `CLI:UNMATCHED_PAD` warning is printed

### Requirement: Update merges by pin number preserving manual overrides
The `update` command SHALL merge source data into an existing TOML by matching on pin number. `--symbol` SHALL fill `kind` and kind-args only for pins where `kind` is absent or matches the `--default-kind` placeholder; it SHALL NOT overwrite voltages, bandwidths, `nc`, or `notes`. `--footprint` SHALL set/overwrite only the pin's pad geometry (`Pad`) and mechanical pads, rebuilding thermal vias from the footprint each time. Pins present in the source but not the existing TOML SHALL be appended with a `CLI:NEW_PIN` warning.

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

### Requirement: part-codegen exposes TOML manifest schema
`part-codegen` SHALL expose the TOML manifest schema with both `Serialize` and `Deserialize` support, mapped directly onto core types: `ComponentMeta` for the `[component]` table, pin definitions embedding core `Pad`/`SymPin` geometry, and core `Pad` for mechanical pads (see the `component-metadata` and `pad-model` specs). The schema SHALL include the honoured `nc` flag. `part-codegen` SHALL expose a `validate(manifest) -> Vec<Diagnostic>` function that checks schema validity, duplicate pin names, unresolved power pins, and pin-name-to-const sanity, driven by a single per-kind required-fields table shared with the builder-expression generator and the CLI's TODO-comment logic.

#### Scenario: CLI serialises a Manifest to TOML
- **WHEN** the CLI builds a `Manifest` from symbol data and serialises it
- **THEN** the output TOML is deserialisable by the same schema without errors

#### Scenario: validate flags unresolved power pins
- **WHEN** a `Manifest` has a pin with `kind = "pwr"` and no `v_min`/`v_max`/`i_max`
- **THEN** `validate()` returns a `VALIDATE:UNRESOLVED_POWER` diagnostic

#### Scenario: Required-field rules live in one place
- **WHEN** a new pin kind is added or a kind's required fields change
- **THEN** exactly one table is updated — validation, codegen errors, and TODO comments all derive from it

### Requirement: builder_expr emits physical fields
`part-codegen`'s builder-expression generation SHALL emit `.pad(...)`/`.symbol(...)` construction matching the core `Pad`/`SymPin` types (see the `pad-model` spec) when the pin definition carries geometry, and SHALL emit the `nc` marker when set. Physical fields SHALL NOT be emitted as individual builder setters.

#### Scenario: Pin with pad geometry round-trips through codegen
- **WHEN** a TOML pin carries pad geometry
- **THEN** the generated Rust constructs the equivalent core `Pad` attached via `.pad(...)`

#### Scenario: nc pin emits the nc marker
- **WHEN** a TOML pin has `nc = true`
- **THEN** the generated Rust marks the pin no-connect and ERC treats it as such

## ADDED Requirements

### Requirement: TOML serialisation derives from the schema
Manifest serialisation SHALL use the derived serde implementation of the schema as the single serialisation path; hand-written per-field TOML writing SHALL be removed. `# TODO: fill …` guidance for incomplete power pins SHALL be produced by a post-pass over the serialised output. Serialise→deserialise SHALL round-trip losslessly for every shipped parts TOML.

#### Scenario: Round-trip is lossless
- **WHEN** any parts-crate TOML is deserialised and serialised again
- **THEN** deserialising the output yields an equal manifest

### Requirement: new and update share one merge pipeline
The `new` and `update` commands SHALL share a single merge implementation: `new` SHALL behave as a merge into an empty manifest, `update` as a merge into the existing manifest. Source handling (symbol, footprint, datasheet, 3D model) SHALL be uniform across both commands, including extension guards, lib-id resolution, and 3D-model embedding.

#### Scenario: Guards behave identically on both commands
- **WHEN** a `.kicad_mod` file is passed to `--symbol` (or a `.kicad_sym` file to `--footprint`) on either `new` or `update`
- **THEN** the same `CLI:FOOTPRINT_AS_SYMBOL`/`CLI:SYMBOL_AS_FOOTPRINT` error is produced by the same code path
