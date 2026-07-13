# compile-report Specification

## Purpose
TBD - created by archiving change core-redesign. Update Purpose after archive.
## Requirements
### Requirement: CompileReport contains the compiled board and warnings
`Board::compile()` SHALL return `Result<CompileReport, CompileError>` on success. `CompileReport` SHALL contain `board: CompiledBoard`, `warnings: Vec<Diagnostic>`, and `summary: CompileSummary`. Warnings SHALL NOT block compilation.

#### Scenario: Successful compilation returns report
- **WHEN** a board with no errors and one floating-input warning is compiled
- **THEN** `CompileReport` is returned with `warnings` containing one diagnostic
- **AND** `board` contains the fully resolved `CompiledBoard`

#### Scenario: Report with no warnings
- **WHEN** a board with no errors and no warnings is compiled
- **THEN** `CompileReport` is returned with `warnings` being empty

### Requirement: CompileSummary provides auditable compiler decisions
`CompileSummary` SHALL contain `nets: Vec<NetInfo>` (inferred net names and properties), `caps_synthesised: Vec<SynthCap>` (decoupling caps added by the compiler), `pin_count: usize`, and `component_count: usize`. The developer SHALL be able to inspect this to audit what the compiler did.

#### Scenario: Summary lists synthesised caps
- **WHEN** a board with a component requiring decoupling is compiled
- **THEN** `summary.caps_synthesised` contains entries for each synthesised capacitor with refdes, value, net, and source pin

#### Scenario: Summary lists inferred nets
- **WHEN** a board with three connections is compiled
- **THEN** `summary.nets` contains entries for each inferred net with name and voltage (or signal type)

### Requirement: CompileError carries all diagnostics
`CompileError` SHALL contain `errors: Vec<Diagnostic>`. When `compile()` encounters multiple errors, ALL errors SHALL be collected and returned together, not just the first. The developer SHALL see every problem in one compilation pass.

#### Scenario: Multiple errors returned together
- **WHEN** a board has two overvoltage errors on different pins
- **THEN** `CompileError.errors` contains two diagnostics
- **AND** both name their respective pins and nets

#### Scenario: Single error
- **WHEN** a board has one overvoltage error
- **THEN** `CompileError.errors` contains exactly one diagnostic

### Requirement: SynthCap records synthesised capacitor details
`SynthCap` SHALL contain `refdes: String`, `value: Qty<Farad>`, `net: String`, `source_component: String`, and `source_pin: String`. This allows the developer to trace which capacitor was added for which pin.

#### Scenario: Synthesised cap traces to source pin
- **WHEN** the compiler synthesises a 100nF cap for U1's VDD pin on net V3V3
- **THEN** `SynthCap` has `refdes == "C1"`, `value == 100nF`, `net == "V3V3"`, `source_component == "U1"`, `source_pin == "VDD"`

