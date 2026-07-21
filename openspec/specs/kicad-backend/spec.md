# kicad-backend Specification

## Purpose
The KiCad backend converts a [`CompiledBoard`] into a complete KiCad 10 project — `.kicad_pro` (project), `.kicad_sch` (schematic), `.kicad_pcb` (PCB), and `.net` (netlist). All symbol and footprint geometry is auto-generated from component pin/pad data; the backend does not require external library files. A separate CLI path renders standalone `.kicad_sym`/`.kicad_mod` files from part TOML manifests for use as project-local libraries.
## Requirements
### Requirement: KiCad backend implements Backend trait
A `KiCad` struct SHALL implement the `Backend` trait. `KiCad::emit(output_dir, &CompiledBoard)` SHALL write a complete KiCad project to the output directory: `.kicad_pro` (project), `.kicad_sch` (schematic), `.kicad_pcb` (PCB), and `.net` (netlist).

#### Scenario: Emit writes all project files
- **WHEN** `KiCad::new().emit("output/", &compiled_board)` is called
- **THEN** the `output/` directory contains `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, and `.net` files

#### Scenario: Emit creates output directory if it does not exist
- **WHEN** the output directory does not exist
- **THEN** `emit()` creates it before writing files

### Requirement: Schematic emitter generates symbols from pin roles
The schematic emitter SHALL generate symbol geometry algorithmically from each component's pin roles and metadata (via [`crate::sym_layout`]). Power pins are placed across the top, ground/thermal pins across the bottom, and signal pins split between left and right edges on the 2.54 mm grid (KLC S4.2). The emitter SHALL NOT open any files to resolve symbols.

#### Scenario: Symbols embedded in lib_symbols
- **WHEN** a compiled board is emitted to schematic
- **THEN** the `.kicad_sch` file's `lib_symbols` section contains one symbol definition per unique symbol identifier, with pins placed by role

#### Scenario: Pin count matches PCB pad count
- **WHEN** a component has thermal vias or mechanical pads
- **THEN** the schematic symbol includes MECH<n> pins matching each non-electrical pad

### Requirement: Netlist emitter produces KiCad netlist format
The netlist emitter SHALL produce a `(export (version "E") ...)` S-expression with `(components ...)` and `(nets ...)` sections. Each component SHALL appear as a `(comp (ref "U1") (value "..."))` entry. Each net SHALL appear as a `(net (code N) (name "...") (node (ref) (pin) (pinfunction) (pintype)))` entry with pin type mapped from `Role`.

#### Scenario: Netlist contains components and nets
- **WHEN** a compiled board with 2 components and 3 nets is emitted
- **THEN** the `.net` file contains 2 `(comp ...)` entries and 3 `(net ...)` entries

### Requirement: PCB emitter produces KiCad PCB format
The PCB emitter SHALL produce a `(kicad_pcb ...)` S-expression with layer table, setup, net classes, and footprint placements. Net classes SHALL be derived from `NetClass` constraints. Footprints SHALL use embedded footprint data when present, falling back to generic rectangular footprints. Components SHALL be auto-placed on a grid for a rough starting point.

#### Scenario: PCB file has correct structure
- **WHEN** a compiled board is emitted to PCB
- **THEN** the `.kicad_pcb` file starts with `(kicad_pcb (version`
- **AND** contains a `layers` section, `setup` section, and at least one `footprint` entry

#### Scenario: Net classes derived from constraints
- **WHEN** a net has `NetClass { min_width: 0.5.mm(), clearance: 0.2.mm() }`
- **THEN** the `.kicad_pcb` file contains a net class with those dimensions

### Requirement: Project emitter produces KiCad project file
The project emitter SHALL produce a `.kicad_pro` JSON file with schema versions matching KiCad 10 expectations, including a `Default` net class.

#### Scenario: Project file is valid JSON
- **WHEN** a compiled board is emitted
- **THEN** the `.kicad_pro` file is valid JSON parseable by KiCad
- **AND** contains `meta.version: 3` and a `Default` net class entry

### Requirement: S-expression parser is brought across from main/
An S-expression parser SHALL be available in the KiCad backend crate for use by the future generator CLI. It SHALL parse S-expression text into a `Sexpr` tree (lists, atoms, strings, comments). It SHALL be the same parser as `main/`'s `sexpr` module.

#### Scenario: Parse round-trips
- **WHEN** an S-expression string is parsed and re-serialised
- **THEN** the output matches the input (modulo whitespace)

### Requirement: Symbol parser is brought across from main/
A `sym_parser` module SHALL be available in the KiCad backend crate for use by the future generator CLI. It SHALL parse `.kicad_sym` files and extract `SymbolDef` structs with pin definitions. It SHALL NOT be used during `Board::compile()` or `Backend::emit()`.

#### Scenario: Symbol parser extracts pin definitions
- **WHEN** a `.kicad_sym` file is parsed by `sym_parser`
- **THEN** `SymbolDef` structs are returned with pin names, numbers, positions, rotations, and lengths

### Requirement: Footprint parser is brought across for CLI use
A `fp_parser` module SHALL be available in the KiCad backend crate for use by the parts-creation CLI. It SHALL parse `.kicad_mod` footprint files and extract `PadDef` structs with pad numbers, positions, rotations, and dimensions. It SHALL NOT be used during `Board::compile()` or `Backend::emit()`.

#### Scenario: Footprint parser extracts pad definitions
- **WHEN** a `.kicad_mod` file is parsed by `fp_parser`
- **THEN** `PadDef` structs are returned with pad numbers, positions, rotations, widths, heights, and pad types

