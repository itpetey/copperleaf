# board-compile-pipeline Specification

## Purpose
TBD - created by archiving change core-redesign. Update Purpose after archive.
## Requirements
### Requirement: Board is the mutable design builder
The `Board` struct SHALL be the mutable, in-progress design. It SHALL store components and connections. `Board::new()` SHALL create an empty board. `Board::add(name, component)` SHALL add a component and return a `ComponentHandle`. `Board::connect(from, to)` SHALL connect two `PinHandle`s and return a `NetHandle`. `Board::compile(self)` SHALL consume the board, run ERC and synthesis, and return `Result<CompileReport, CompileError>`.

#### Scenario: Create empty board
- **WHEN** `Board::new()` is called
- **THEN** the resulting board has no components and no connections

#### Scenario: Add component returns handle
- **WHEN** `board.add("rpi", Rp2354a::new())` is called
- **THEN** a `ComponentHandle` is returned that can be used to reference pins on that component

#### Scenario: Connect two pins
- **WHEN** `board.connect(rpi.pin(Rp2354a::IOVDD), radio.pin(Mm8108::VBAT))` is called
- **THEN** a `NetHandle` is returned representing the emerging net
- **AND** the connection is recorded for net inference during compilation

#### Scenario: Compile consumes the board
- **WHEN** `board.compile()` is called on a board with no errors
- **THEN** a `CompileReport` containing a `CompiledBoard` is returned
- **AND** the original `Board` is no longer usable (consumed by `compile()`)

### Requirement: CompiledBoard is the immutable verified artifact
The `CompiledBoard` struct SHALL be immutable and contain all resolved data: components (original + synthesised) with deterministic refdes, inferred nets with properties, resolved connections, and all constraints.

#### Scenario: CompiledBoard carries synthesised components
- **WHEN** a board with a component requiring decoupling is compiled
- **THEN** the `CompiledBoard` contains both the original components and the synthesised decoupling capacitors with deterministic refdes

### Requirement: Backend trait provides emission interface
A `Backend` trait SHALL define `fn emit(&self, output_dir: &str, board: &CompiledBoard) -> Result<(), Self::Error>`. Backends (KiCad, future SPICE, etc.) SHALL implement this trait. The developer's `main.rs` SHALL construct a backend and call `emit()` with a `CompiledBoard`.

#### Scenario: KiCad backend emits to filesystem
- **WHEN** `KiCad::new().emit("output/", &compiled_board)` is called
- **THEN** KiCad project files (`.kicad_sch`, `.kicad_pcb`, `.kicad_pro`, `.net`) are written to the `output/` directory

#### Scenario: main.rs is the sole source of truth
- **WHEN** a project's `main.rs` builds a board, compiles it, and calls `backend.emit()`
- **THEN** `cargo run` produces the complete backend output with no intermediary files or CLI steps

### Requirement: Compile runs ERC and blocks on errors
`Board::compile()` SHALL run all ERC rules before producing a `CompiledBoard`. If any ERC rule produces an error-severity diagnostic, `compile()` SHALL return `CompileError` with all errors. Warnings SHALL NOT block compilation and SHALL be included in `CompileReport`.

#### Scenario: ERC error blocks compilation
- **WHEN** a board has a pin connected to a net with voltage exceeding the pin's `v_max`
- **THEN** `compile()` returns `CompileError` containing the overvoltage diagnostic

#### Scenario: ERC warning does not block
- **WHEN** a board has a floating input pin (warning severity)
- **THEN** `compile()` returns `CompileReport` with the warning in `report.warnings`
- **AND** the `CompiledBoard` is still produced

