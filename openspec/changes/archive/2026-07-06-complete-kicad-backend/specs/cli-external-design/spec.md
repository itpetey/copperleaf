## MODIFIED Requirements

### Requirement: CLI subcommands accept optional design JSON path
All CLI subcommands (`verify`, `export`, `export-sch`, `export-pcb`, `json`, `decouple`, `report`) SHALL accept an optional positional `<design.json>` argument. When provided, the design SHALL be loaded from the file. When omitted, the built-in example design SHALL be used.

#### Scenario: Verify with external design file
- **WHEN** the user runs `cl verify my_design.json`
- **THEN** the design is loaded from `my_design.json` and ERC is run on it

#### Scenario: Verify with no argument (backwards compat)
- **WHEN** the user runs `cl verify` with no arguments
- **THEN** the built-in example design is used (same behavior as before)

#### Scenario: File not found
- **WHEN** the user runs `cl verify nonexistent.json`
- **THEN** a clear error message is printed to stderr and the process exits non-zero

#### Scenario: Export-sch with external design file
- **WHEN** the user runs `cl export-sch my_design.json`
- **THEN** the design is loaded from `my_design.json` and a KiCad schematic is written to stdout

#### Scenario: Export-pcb with no argument
- **WHEN** the user runs `cl export-pcb` with no arguments
- **THEN** the built-in example design is used and a KiCad PCB file is written to stdout

## ADDED Requirements

### Requirement: Export subcommands emit KiCad files
The `export` subcommand SHALL emit a KiCad S-expression netlist via `backend_kicad::emit_netlist`. The `export-sch` subcommand SHALL emit a KiCad schematic via `backend_kicad::emit_schematic`. The `export-pcb` subcommand SHALL emit a KiCad PCB file via `backend_kicad::emit_pcb`. Each SHALL write the emitter output to stdout.

#### Scenario: Export emits a KiCad netlist
- **WHEN** the user runs `cl export`
- **THEN** stdout begins with `(export` and contains `(components` and `(nets` sections

#### Scenario: Export-sch emits a KiCad schematic
- **WHEN** the user runs `cl export-sch`
- **THEN** stdout begins with `(kicad_sch`

#### Scenario: Export-pcb emits a KiCad PCB
- **WHEN** the user runs `cl export-pcb`
- **THEN** stdout begins with `(kicad_pcb` and contains `(net_class "Default"`
