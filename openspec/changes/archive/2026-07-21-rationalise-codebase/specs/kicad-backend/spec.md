## MODIFIED Requirements

### Requirement: Schematic emitter generates symbols from pin data
The schematic emitter SHALL generate each component's symbol from its pin data (roles, names, numbers) via the shared functional auto-layout: positive power pins across the top, ground/thermal pins across the bottom, remaining signals split left/right, all pin connection points on the 2.54 mm grid. Thermal vias and mechanical pads SHALL appear as additional `MECH<n>` symbol pins, synthesised by a single shared helper so that schematic pin count always matches PCB pad count. The emitter SHALL NOT open any files to resolve symbols.

#### Scenario: Power and ground pins placed conventionally
- **WHEN** a component has a `PowerIn` pin "VDD" and a `Gnd` pin "GND"
- **THEN** the embedded symbol places VDD on the top edge and GND on the bottom edge of the symbol body

#### Scenario: Thermal vias become MECH pins
- **WHEN** a component has two thermal vias and one mechanical mounting pad
- **THEN** the schematic symbol contains three additional `MECH1..3` pins whose numbers match the corresponding PCB pads

#### Scenario: No files opened during emission
- **WHEN** a compiled board is emitted
- **THEN** no symbol or footprint library files are read from disk

### Requirement: One emission path for symbols and footprints
Symbol and footprint S-expression generation SHALL be implemented once in the backend, operating on the unified component representation (`ComponentMeta` + pins with `Pad`/`SymPin` data). The CLI `generate symbol`/`generate footprint` commands, the schematic's embedded `lib_symbols` section, the PCB's embedded footprints, and any standalone library emission SHALL all use these same functions. Pad geometry in every case SHALL be resolved by the core `resolve_pad()` rules (see the `pad-model` spec).

#### Scenario: Standalone and embedded footprints are identical
- **WHEN** a part is emitted standalone via `generate footprint` and embedded via board emission
- **THEN** the pad geometry, outlines, and text items in both outputs agree

#### Scenario: Property nodes built by one helper
- **WHEN** any emitter produces a KiCad `(property ...)` node
- **THEN** it is constructed by a single shared helper — no per-module property builders exist

### Requirement: Backend output is byte-identical across processes
Given the same `CompiledBoard`, the backend SHALL produce byte-identical `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, and `.net` files in separate operating-system processes. No iteration over std `HashMap` (or other randomised-order structures) SHALL occur on any output path; ordered maps (`BTreeMap`/`IndexMap`) or sorted vectors SHALL be used instead.

#### Scenario: Two processes emit identical files
- **WHEN** the same board is compiled and emitted in two separate processes
- **THEN** all four output files are byte-for-byte identical
