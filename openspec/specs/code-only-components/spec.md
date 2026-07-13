# code-only-components Specification

## Purpose
TBD - created by archiving change core-redesign. Update Purpose after archive.
## Requirements
### Requirement: Component trait carries embedded physical data
The `Component` trait SHALL provide `symbol() -> Option<&'static str>` and `footprint() -> Option<&'static str>` methods with default implementations returning `None`. These methods SHALL return the actual S-expression content embedded in the binary, not file paths. No method on `Component` or any other type SHALL accept or return filesystem paths for symbol or footprint resolution.

#### Scenario: Component with embedded symbol
- **WHEN** a component's `symbol()` returns `Some("(symbol \"RP2354a\" ...)")`
- **THEN** the backend receives the S-expression content directly without any file I/O

#### Scenario: Component without symbol
- **WHEN** a component does not override `symbol()`
- **THEN** `symbol()` returns `None` and the backend uses a generic placeholder

### Requirement: No filesystem dependencies in component resolution
The compiler (`Board::compile()`) SHALL NOT perform any filesystem operations to resolve symbols, footprints, or pin positions. Pin positions (`pos`, `rotation`, `length`) SHALL be stored on `Pin` at component construction time. The backend SHALL use embedded data from `CompiledBoard` for emission.

#### Scenario: Compile succeeds without any filesystem access
- **WHEN** `board.compile()` is called on a board with components that have embedded symbols
- **THEN** no files are read from disk during compilation
- **AND** pin positions are read from `Pin` fields, not parsed from files

#### Scenario: Backend emits without reading symbol files
- **WHEN** a backend emits a `CompiledBoard` whose components have embedded symbols
- **THEN** the backend splices the embedded S-expression content into the output
- **AND** no symbol library files are opened

### Requirement: Component trait is backend-agnostic
The `Component` trait method names SHALL NOT reference any specific backend (e.g. no `kicad_symbol()`). Methods SHALL be named generically (`symbol()`, `footprint()`). The payload format is an implementation detail that may change when additional backends are added.

#### Scenario: Trait methods are generically named
- **WHEN** the `Component` trait is inspected
- **THEN** it contains `symbol()` and `footprint()`, not `kicad_symbol()` or `kicad_footprint()`

### Requirement: Component trait carries constraints for synthesis
The `Component` trait SHALL provide `fn constraints(&self) -> Vec<Constraint>` with a default implementation returning an empty vec. Components SHALL use this to declare decoupling requirements, impedance targets, length-match groups, and other constraints that drive synthesis and analysis during compilation.

#### Scenario: Component declares decoupling constraint
- **WHEN** a component's `constraints()` returns `vec![Constraint::Decoupling { values: [100.0.nf(), 1.0.uf()], per_pin: true }]`
- **THEN** the compiler synthesises decoupling capacitors for the component's power pins during compilation

#### Scenario: Component with no constraints
- **WHEN** a component does not override `constraints()`
- **THEN** `constraints()` returns an empty vec and no synthesis is triggered for that component

