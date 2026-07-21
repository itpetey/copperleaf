## ADDED Requirements

### Requirement: ComponentMeta is the single component metadata struct
A `ComponentMeta` struct SHALL be defined in the core crate carrying all non-electrical component metadata: `symbol`, `footprint`, `datasheet`, `description`, `model_3d`, `model_3d_data`, `model_3d_rotation`, and `model_3d_offset`. No other struct in the workspace SHALL re-declare these fields individually (including the codegen `ComponentMeta`, which SHALL be replaced by the core type).

#### Scenario: Metadata flows from TOML to compiled board unchanged
- **WHEN** a parts TOML declares `datasheet` and `model_3d`
- **THEN** the generated component's `meta()` returns those values and the `CompiledComponent` carries them through compilation unmodified

### Requirement: Component trait collapses to cohesive methods
The `Component` trait SHALL expose `pins()`, `meta()`, `mechanical()`, and `constraints()` — and SHALL NOT expose per-field getters (`symbol()`, `footprint()`, `datasheet()`, `description()`, `model_3d()`, `model_3d_data()`, `model_3d_rotation()`, `model_3d_offset()`). `meta()` SHALL have a default implementation returning an empty `ComponentMeta`.

#### Scenario: Minimal component compiles with one method
- **WHEN** a hand-written component implements only `pins()`
- **THEN** it satisfies the `Component` trait and its metadata is treated as empty

### Requirement: CompiledComponent embeds ComponentMeta
`CompiledComponent` SHALL consist of `refdes`, `meta: ComponentMeta`, `pins`, `mechanical: Vec<Pad>`, and `constraints` — no flattened per-field metadata. A single `CompiledComponent::from_component(refdes, &dyn Component)` constructor SHALL be the only trait→compiled conversion, used by both board lowering and decoupling-capacitor synthesis.

#### Scenario: Synthesised and lowered components share one constructor
- **WHEN** the compiler lowers board components and synthesises a decoupling capacitor
- **THEN** both are produced by `CompiledComponent::from_component` with no duplicated field-copying code

### Requirement: Manifest maps onto core types
The parts TOML `Manifest` SHALL serde-(de)serialise directly onto core types: `ComponentMeta` for the `[component]` table, core-shaped pin definitions embedding `Pad`/`SymPin`, and `Pad` for mechanical pads. The schema SHALL remain the single source of truth for serialisation — there SHALL be one serialisation implementation (derived), with CLI comment injection (`# TODO: fill …`) implemented as a post-pass.

#### Scenario: TOML round-trips through the schema
- **WHEN** a parts TOML is deserialised into a `Manifest` and serialised again
- **THEN** the output contains the same pins, geometry, constraints, and metadata with no hand-written serialiser involved

### Requirement: One emission path for symbols and footprints
The KiCad backend SHALL expose one symbol emitter and one footprint emitter operating on the unified component representation. The CLI `generate symbol|footprint` commands and board emission (`.kicad_sch` `lib_symbols`, `.kicad_pcb` footprints) SHALL use these same functions. There SHALL be no separate manifest-based and compiled-component-based emitter pairs.

#### Scenario: Standalone and embedded footprints match
- **WHEN** `generate footprint` emits a part TOML and a board containing that part is emitted
- **THEN** both footprints are produced by the same emitter function and agree in pads, outlines, and text items
