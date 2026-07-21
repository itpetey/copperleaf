## MODIFIED Requirements

### Requirement: Component trait carries embedded physical data
The `Component` trait SHALL expose component metadata through a single `meta() -> &ComponentMeta` method (see the `component-metadata` spec) with a default implementation returning an empty `ComponentMeta`. `symbol` and `footprint` within `ComponentMeta` SHALL be library identifiers (e.g. `"RP2354A"` or `"MCU_RaspberryPi:RP2354A"`), and pin physical data SHALL be embedded as `Pad`/`SymPin` values on the component's `Pin`s (see the `pad-model` spec). No method on `Component` or any other type SHALL accept or return filesystem paths for symbol or footprint resolution.

#### Scenario: Component with a project-local symbol identifier
- **WHEN** a component's `meta().symbol` is `Some("RP2354A")`
- **THEN** the backend emits the component's symbol into the project's own symbol library without any file I/O

#### Scenario: Component without metadata
- **WHEN** a component does not override `meta()`
- **THEN** `meta()` returns an empty `ComponentMeta` and the backend falls back to generic symbol/footprint identifiers derived from the refdes

### Requirement: No filesystem dependencies in component resolution
The compiler (`copperleaf_compile::run`) SHALL NOT perform any filesystem operations to resolve symbols, footprints, or pin positions. Pad and symbol-pin geometry SHALL be stored on `Pin` (as `Pad`/`SymPin`) at component construction time. The backend SHALL use embedded data from `CompiledBoard` for emission.

#### Scenario: Compile succeeds without any filesystem access
- **WHEN** a board is compiled whose components carry embedded pad/symbol data
- **THEN** no files are read from disk during compilation
- **AND** pin positions are read from `Pin` fields, not parsed from files

#### Scenario: Backend emits without reading symbol files
- **WHEN** a backend emits a `CompiledBoard`
- **THEN** all symbol and footprint geometry is generated from embedded data
- **AND** no symbol library files are opened
