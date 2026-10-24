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
The netlist emitter SHALL produce a `(export (version "E") ...)` S-expression with `(components ...)` and `(nets ...)` sections. Each component SHALL appear as a `(comp (ref "U1") (value "..."))` entry containing every electrical physical pin. Each net SHALL contain one node per connected physical pin, with its own pad number, function name, and mapped pin type. Repeated pin names SHALL NOT collapse into one node.

#### Scenario: Netlist contains components and nets
- **WHEN** a compiled board with 2 components and 3 nets is emitted
- **THEN** the `.net` file contains 2 `(comp ...)` entries and 3 `(net ...)` entries

#### Scenario: Repeated-name pads produce separate nodes
- **WHEN** six `AVDD` pads are connected to one net
- **THEN** that net contains six nodes with distinct pad numbers
- **AND** none is omitted because another pin shares its name

### Requirement: PCB emitter produces KiCad PCB format
The PCB emitter SHALL produce a `(kicad_pcb ...)` S-expression with a layer table, setup, net classes, and footprint placements. Net classes SHALL derive from each net's resolved class. Footprints SHALL use embedded footprint data when present and otherwise use a generic rectangular footprint. Without a layout, components SHALL be auto-placed and no copper emitted. With a layout, placements SHALL come from the layout.

#### Scenario: PCB file has correct structure
- **WHEN** a compiled board is emitted to PCB
- **THEN** the `.kicad_pcb` file starts with `(kicad_pcb (version`
- **AND** contains a `layers` section, `setup` section, and at least one `footprint` entry

#### Scenario: Net classes derived from constraints
- **WHEN** a net has a resolved class with minimum width `0.5 mm` and clearance `0.2 mm`
- **THEN** the `.kicad_pcb` file contains a net class with those dimensions

### Requirement: Emitted views agree per physical pad
For every physical electrical pin, the schematic, netlist, and PCB SHALL agree whether it is connected and which net owns it. Every connected pad SHALL emit its resolved net in the PCB and exactly one node in its netlist net. Every unconnected electrical pad SHALL emit no PCB net. The schematic SHALL attach its wire or label to that exact pad.

#### Scenario: Every ground pad agrees across outputs
- **WHEN** a W5500 component has multiple physical ground pads connected to one net
- **THEN** every one of those pads emits that net in the PCB
- **AND** every one appears as a node in the netlist
- **AND** the schematic addresses every one independently

#### Scenario: Unconnected pad has no phantom net
- **WHEN** only one of two same-named pads is connected
- **THEN** the unconnected pad emits no net in the PCB
- **AND** does not appear as a node in the netlist

### Requirement: Back-side footprints are fully transformed
A footprint placed on the back side SHALL emit its body on `B.Cu`, every SMD copper pad on `B.Cu`, and its corresponding mask and paste layers on `B.Mask` and `B.Paste`. Fabrication, silkscreen, and courtyard graphics and text SHALL use their back-side layers with the geometry and text orientation required by KiCad. No front-side copper or graphical layer SHALL remain for a back-side footprint.

#### Scenario: Back-side SMD pads use back copper
- **WHEN** an SMD footprint is placed with `BoardSide::Back`
- **THEN** its footprint and every pad use back-side copper and mask layers
- **AND** front-side copper and paste layers are absent

#### Scenario: Back-side graphics use back layers
- **WHEN** a footprint is placed on the back side
- **THEN** silkscreen, fabrication, courtyard, and visible text use back-side layers
- **AND** their orientation matches the mirrored footprint

### Requirement: Copper geometry retains design-rule precision
Copper tracks, vias, widths, and zone coordinates SHALL be emitted at KiCad's full supported coordinate precision after design-rule checking. The rounded coordinate used for footprint courtyard geometry SHALL NOT be applied to copper. Validation or design-rule checks SHALL evaluate the same coordinates that are emitted.

#### Scenario: Fine-pitch copper is not quantised after DRC
- **WHEN** a track endpoint or width has a coordinate below `0.01 mm` resolution
- **THEN** the emitted PCB preserves that value within KiCad's supported precision
- **AND** the value checked by DRC equals the emitted value

#### Scenario: Canonical layer maps to KiCad copper
- **WHEN** layout geometry uses canonical back-copper ordinal `1` on a two-layer board
- **THEN** the PCB emits that geometry on `B.Cu`
- **AND** emits no `In1.Cu`

### Requirement: Footprint and 3D-model references are independent
Component metadata SHALL expose separate footprint and 3D-model references. A footprint reference SHALL be a valid KiCad `Library:Footprint` identifier or project-local footprint name, never a `.3dshapes` model path. Embedded model data SHALL be written only when non-empty and SHALL use a safe model filename.

#### Scenario: Passive footprint and model do not collide
- **WHEN** a 0603 resistor is emitted
- **THEN** its footprint reference identifies `Resistor_SMD:R_0603_1608Metric`
- **AND** any 3D-model path identifies a `.step` or `.wrl` model separately

#### Scenario: Empty model data emits no model file
- **WHEN** a component declares empty encoded model data
- **THEN** emission writes no zero-length model file
- **AND** reports a validation diagnostic

### Requirement: Backend output remains within the destination directory
Before writing, the KiCad backend SHALL validate project, footprint, and model filenames derived from board or manifest data. Absolute paths, parent traversal, and paths resolving outside the destination after symlink resolution SHALL be rejected without writing outside it.

#### Scenario: Traversal in project name is rejected
- **WHEN** a project name contains `../` or an absolute path
- **THEN** emission fails with a path-containment error
- **AND** writes no file outside the destination

#### Scenario: Manifest footprint name cannot escape
- **WHEN** a project-local footprint name resolves outside the footprint output directory
- **THEN** emission rejects the name and identifies the component

### Requirement: Preserved-board updates remove stale net references
Updating an existing PCB from a new compiled board SHALL remove or explicitly clear nets that no longer exist. Preserved pads SHALL NOT retain an old numeric net code that could alias a different new net. Renamed nets SHALL preserve copper, and deleted nets SHALL not survive through code reuse.

#### Scenario: Deleted net is cleared from preserved pads
- **WHEN** a preserved footprint pad references a net removed from the new board
- **THEN** that pad emits no net after update

#### Scenario: Reused net code does not alias old copper
- **WHEN** a deleted net's numeric code is reused for a different new net
- **THEN** preserved objects from the deleted net do not attach to the replacement net

#### Scenario: Renamed net preserves copper
- **WHEN** a net is renamed but remains electrically the same
- **THEN** its tracks, vias, zones, and pad associations survive with the new name and code

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

