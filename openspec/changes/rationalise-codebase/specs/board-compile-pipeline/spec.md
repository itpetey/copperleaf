## MODIFIED Requirements

### Requirement: Board is the mutable design builder
The `Board` struct SHALL be the mutable, in-progress design. It SHALL store components, connections, single-pin nets, and net overrides. `Board::new()` SHALL create an empty board. `Board::add(name, component)` SHALL add a component and return a `ComponentHandle`. `Board::connect(from, to)` SHALL connect two `PinHandle`s and return a `NetHandle`. `Board::net(pin)` SHALL register a single-pin net and return a `NetHandle`. `copperleaf_compile::run(board, options)` SHALL consume the board, run ERC and synthesis, and return `Result<CompileReport, CompileError>`.

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
- **WHEN** a board with no errors is compiled
- **THEN** a `CompileReport` containing a `CompiledBoard` is returned
- **AND** the original `Board` is no longer usable (consumed by compilation)

### Requirement: CompiledBoard is the immutable verified artifact
The `CompiledBoard` struct SHALL be immutable and contain all resolved data: components (original + synthesised) with deterministic refdes, inferred nets with properties, resolved connections, board dimensions, and all constraints. Each `CompiledComponent` SHALL consist of `refdes`, `meta: ComponentMeta`, `pins`, `mechanical: Vec<Pad>`, and `constraints`. Each `Connection` SHALL reference its component by index and its net by index into `nets`. A single `CompileError` type SHALL be defined in the core crate and used by board building, compilation, and validation.

#### Scenario: CompiledBoard carries synthesised components
- **WHEN** a board with a component requiring decoupling is compiled
- **THEN** the `CompiledBoard` contains both the original components and the synthesised decoupling capacitors with deterministic refdes

#### Scenario: One CompileError type
- **WHEN** any of `Board::connect`, `Board::net`, or compilation fails
- **THEN** the error is the same `copperleaf::CompileError` type carrying `Vec<Diagnostic>`

## ADDED Requirements

### Requirement: Compilation provides a precomputed board view
The compiled artifact SHALL make connectivity lookups available without re-scanning connections: given a `(component, pin)` pair, consumers SHALL be able to obtain the owning net (or absence) in better than linear time. ERC rules and backend emitters SHALL use this view rather than rebuilding per-consumer lookup maps.

#### Scenario: ERC and emitters share one connectivity index
- **WHEN** ERC rules and the KiCad emitters run against the same compiled board
- **THEN** none of them construct their own pin→net or refdes→component maps — all connectivity queries go through the shared view
