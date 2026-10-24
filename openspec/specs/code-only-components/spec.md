# code-only-components Specification

## Purpose
Define how components expose their symbol, footprint, and metadata from generated code, so compilation and emission need no filesystem library resolution.

## Requirements

### Requirement: Component metadata is embedded in the binary
The `Component` trait SHALL expose `meta() -> &ComponentMeta` with a default implementation returning `ComponentMeta::EMPTY`. Metadata SHALL carry the symbol identifier, footprint identifier, datasheet URL, description, and 3D-model data as owned values. No method on `Component` or any other type SHALL accept or return filesystem paths for symbol or footprint resolution during compilation or emission.

#### Scenario: Component with metadata
- **WHEN** a component's `meta()` returns a `ComponentMeta` with a footprint identifier
- **THEN** the backend receives that identifier directly without any file I/O

#### Scenario: Component without metadata
- **WHEN** a component does not override `meta()`
- **THEN** `meta()` returns `ComponentMeta::EMPTY` and the backend uses a generic placeholder

### Requirement: No filesystem dependencies in component resolution
The compiler (`copperleaf_compile::run()`) SHALL NOT perform any filesystem operations to resolve symbols, footprints, or pin positions. Pin positions (`pos`, `rotation`, `length`) SHALL be stored on `Pin` at component construction time. The backend SHALL use embedded data from `CompiledBoard` for emission.

#### Scenario: Compile succeeds without any filesystem access
- **WHEN** `copperleaf_compile::run()` is called on a board with components that have embedded metadata
- **THEN** no files are read from disk during compilation
- **AND** pin positions are read from `Pin` fields, not parsed from files

#### Scenario: Backend emits without reading symbol files
- **WHEN** a backend emits a `CompiledBoard` whose components carry embedded metadata
- **THEN** the backend uses the metadata identifiers directly
- **AND** no symbol library files are opened

### Requirement: Component trait is backend-agnostic
The `Component` trait method names SHALL NOT reference any specific backend (e.g. no `kicad_symbol()`). Methods SHALL be named generically (`pins()`, `meta()`, `mechanical()`, `constraints()`, `layout_constraints()`). The payload format is an implementation detail that may change when additional backends are added.

#### Scenario: Trait methods are generically named
- **WHEN** the `Component` trait is inspected
- **THEN** it contains generic methods rather than `kicad_symbol()` or `kicad_footprint()`

### Requirement: Component trait carries constraints for synthesis
The `Component` trait SHALL provide `fn constraints(&self) -> Vec<Constraint>` and `fn layout_constraints(&self) -> Vec<LayoutConstraint>`, each with a default implementation returning an empty vec. Components SHALL use these to declare decoupling requirements, impedance targets, length-match groups, and physical layout directives that drive synthesis, analysis, and layout.

#### Scenario: Component declares decoupling constraint
- **WHEN** a component's `constraints()` returns `vec![Constraint::Decoupling { values: [100.0.nf(), 1.0.uf()], per_pin: true }]`
- **THEN** the compiler synthesises decoupling capacitors for the component's power pins during compilation

#### Scenario: Component with no constraints
- **WHEN** a component does not override `constraints()` or `layout_constraints()`
- **THEN** both methods return empty vectors and no synthesis or layout directive is triggered for that component
