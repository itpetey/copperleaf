## ADDED Requirements

### Requirement: CLI subcommands accept optional design JSON path
All CLI subcommands (`verify`, `export`, `json`, `decouple`, `report`) SHALL accept an optional positional `<design.json>` argument. When provided, the design SHALL be loaded from the file. When omitted, the built-in example design SHALL be used.

#### Scenario: Verify with external design file
- **WHEN** the user runs `cl verify my_design.json`
- **THEN** the design is loaded from `my_design.json` and ERC is run on it

#### Scenario: Verify with no argument (backwards compat)
- **WHEN** the user runs `cl verify` with no arguments
- **THEN** the built-in example design is used (same behavior as before)

#### Scenario: File not found
- **WHEN** the user runs `cl verify nonexistent.json`
- **THEN** a clear error message is printed to stderr and the process exits non-zero

### Requirement: Report subcommand produces human-readable design summary
A `report` subcommand SHALL load a design and print a structured text summary including: graph stats (nodes, edges, components, nets, constraints), component list grouped by type, power and signal net summary, ERC results, and decoupling synthesis result.

#### Scenario: Report on a design with components and nets
- **WHEN** `cl report my_design.json` is run on a design with 3 ICs and 5 power nets
- **THEN** the output includes a component list showing 3 ICs
- **AND** the output includes a net summary showing 5 power nets with their voltages
- **AND** the output includes ERC results (overvoltage checks, NC-pin checks)
- **AND** the output includes decoupling synthesis caps

#### Scenario: Report with no argument
- **WHEN** `cl report` is run with no arguments
- **THEN** the built-in example design is reported

### Requirement: Report function is available as a library API
The `copperleaf-analysis` crate SHALL expose a `pub fn report(design: &Design) -> String` function that returns the same human-readable summary as the `report` CLI subcommand. Library consumers SHALL be able to call this function directly.

#### Scenario: Library consumer calls report
- **WHEN** a consumer calls `copperleaf::report(&design)` on a design with components
- **THEN** a `String` is returned containing the formatted summary text

### Requirement: Emit subcommand outputs example design as JSON
An `emit` subcommand SHALL serialize the built-in example design to pretty-printed JSON and write it to stdout. This provides a baseline design file for manual editing or patch application.

#### Scenario: Emit produces valid JSON
- **WHEN** `cl emit` is run
- **THEN** stdout contains valid JSON that can be deserialized back into a `Design`
- **AND** the JSON includes a `"connections"` array (may be empty for the example design)
