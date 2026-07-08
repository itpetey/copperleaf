## Requirements

### Requirement: Netlist emitter produces a KiCad S-expression netlist
The crate SHALL provide `pub fn emit_netlist(design: &Design) -> String` that returns a KiCad interchange netlist. The output SHALL begin with `(export` and contain `(design`, `(components`, and `(nets` sections. The output SHALL be stable across repeated calls on the same design.

#### Scenario: Netlist with components and connections
- **WHEN** `emit_netlist` is called on a design with two components (U1, U2) and connections (U1.VIN→VBUS, U2.VDD→V3V3)
- **THEN** the output contains `comp` entries for U1 and U2 (each with a `ref` matching the refdes)
- **AND** the output contains `net` entries for `VBUS` and `V3V3`
- **AND** each net entry contains `node` entries whose `ref` and `pin` match the connected pins

#### Scenario: Empty design produces a valid netlist
- **WHEN** `emit_netlist` is called on `Design::default()`
- **THEN** the output begins with `(export` and contains empty `(components` and `(nets` sections
- **AND** the output does not contain any `(comp` or `(node` entries

#### Scenario: Netlist output is deterministic
- **WHEN** `emit_netlist` is called twice on the same design
- **THEN** both outputs are byte-for-byte identical

### Requirement: Netlist component value uses refdes prefix
For each component, `emit_netlist` SHALL emit a `comp` entry containing a `ref` matching the refdes and a `value` matching `<prefix>`, where `<prefix>` is the leading alphabetic characters of the refdes (e.g. `U1` → `U`, `C1` → `C`). When the refdes has no alphabetic prefix, the value SHALL be `?`.

#### Scenario: Value derived from refdes prefix
- **WHEN** a design contains components `U1`, `C2`, and `3V3` (no alpha prefix)
- **THEN** the netlist contains a `value` of `U` for `U1`, `C` for `C2`, and `?` for `3V3`

### Requirement: Netlist node pin type is derived from pin role
Each `(node ...)` in the netlist SHALL include a `(pintype "<type>")` derived from the connected pin's `Role` by looking up the pin on the component record. The mapping SHALL be: `PowerIn`/`Gnd` → `power_in`, `PowerOut` → `power_out`, `AnalogIn` → `input`, `AnalogOut` → `output`, `DigitalIO`/`DiffPos`/`DiffNeg` → `bidirectional`. When the pin cannot be found on the component, `pinfunction` and `pintype` SHALL be omitted.

#### Scenario: Power input pin gets power_in pintype
- **WHEN** a connection references a `PowerIn` pin named `VDD` on component `U1`
- **THEN** the netlist node contains a `pinfunction` matching the pin name and a `pintype` of `power_in`

#### Scenario: Digital IO pin gets bidirectional pintype
- **WHEN** a connection references a `DigitalIO` pin
- **THEN** the netlist node contains a `pintype` of `bidirectional`

### Requirement: Schematic emitter produces a valid kicad_sch
The crate SHALL provide `pub fn emit_schematic(design: &Design) -> String` that returns a KiCad 10 schematic. The output SHALL begin with `(kicad_sch` and contain `version`, `generator`, `generator_version`, `uuid`, `lib_symbols`, and `sheet_instances` nodes. The output SHALL be deterministic.

#### Scenario: Schematic with placed components and net labels
- **WHEN** `emit_schematic` is called on a design with component `U1` and connection `U1.VDD→V3V3`
- **THEN** the output contains a `(symbol` instance with `(property "Reference" "U1"`
- **AND** the output contains a `(label "V3V3"` node

#### Scenario: Empty design schematic
- **WHEN** `emit_schematic` is called on `Design::default()`
- **THEN** the output begins with `(kicad_sch` and contains a `sheet_instances` node with no `symbol` instances

### Requirement: PCB emitter produces a valid kicad_pcb with net classes
The crate SHALL provide `pub fn emit_pcb(design: &Design) -> String` that returns a KiCad 10 PCB file. The output SHALL begin with `(kicad_pcb` and contain `version`, `generator`, `generator_version`, a `net` table, at least one `net_class` (always `Default`), and a `footprint` per component. The output SHALL be deterministic.

#### Scenario: Net classes derived from NetClass constraints
- **WHEN** `emit_pcb` is called on a design where a net has `NetClass { min_width: Some(0.3.mm()), clearance: Some(0.2.mm()) }`
- **THEN** the output contains a `net_class` entry whose width is `0.3` and clearance is `0.2` (in mm)
- **AND** that net name appears as a member of that net class

#### Scenario: Default net class always present
- **WHEN** `emit_pcb` is called on any design (including empty)
- **THEN** the output contains `(net_class "Default"`

#### Scenario: Footprint pad carries net assignment
- **WHEN** `emit_pcb` is called on a design with component `U1` and connection `U1.VDD→V3V3`
- **THEN** the output contains a `(footprint` for `U1` with a `(pad` whose `(net` references `V3V3`

#### Scenario: Empty design PCB
- **WHEN** `emit_pcb` is called on `Design::default()`
- **THEN** the output begins with `(kicad_pcb`, contains `(net_class "Default"`, and contains no `footprint` nodes

### Requirement: Emitters use deterministic UUIDs
Where a KiCad file requires a `uuid`, the emitters SHALL produce a deterministic UUID-formatted string (8-4-4-4-12 hex) derived from a stable seed incorporating the entity kind and identifier. The same entity in the same design SHALL always produce the same UUID, and distinct entities SHALL produce distinct UUIDs.

#### Scenario: UUIDs are stable across calls
- **WHEN** `emit_schematic` is called twice on the same design containing `U1`
- **THEN** the UUID string for `U1`'s symbol instance is identical in both outputs

#### Scenario: Distinct entities have distinct UUIDs
- **WHEN** `emit_schematic` is called on a design with `U1` and `U2`
- **THEN** the two symbol instances carry different UUID strings
