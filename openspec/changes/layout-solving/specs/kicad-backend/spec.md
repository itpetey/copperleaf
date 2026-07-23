## MODIFIED Requirements

### Requirement: PCB emitter produces KiCad PCB format
The PCB emitter SHALL produce a `(kicad_pcb ...)` S-expression with layer table, setup, net classes, and footprint placements. Net classes SHALL be derived from each net's resolved `Net.class` (populated from `LayoutConstraint::NetClass` directives during lowering). Footprints SHALL use embedded footprint data when present, falling back to generic rectangular footprints. When no `Layout` is supplied (`emit()`), components SHALL be auto-placed on a grid for a rough starting point and no copper is emitted. When a `Layout` is supplied (`emit_with_layout()`), placements SHALL come from the layout — including rotation and board side — instead of auto-placement.

#### Scenario: PCB file has correct structure
- **WHEN** a compiled board is emitted to PCB
- **THEN** the `.kicad_pcb` file starts with `(kicad_pcb (version`
- **AND** contains a `layers` section, `setup` section, and at least one `footprint` entry

#### Scenario: Net classes derived from resolved net class
- **WHEN** a net carries `LayoutConstraint::NetClass { min_width: 0.5.mm(), clearance: 0.2.mm() }` and is compiled
- **THEN** the `.kicad_pcb` file contains a net class with those dimensions

#### Scenario: Placements come from the layout when supplied
- **WHEN** `emit_with_layout()` is called with a layout placing `J1` at `(10.0, 40.0)` rotated 90°
- **THEN** the `.kicad_pcb` footprint for `J1` has `(at 10 40 90)`
- **AND** no auto-placement position is used

#### Scenario: Emission without layout is unchanged
- **WHEN** `emit()` is called without a layout
- **THEN** the `.kicad_pcb` output is byte-identical to the pre-layout behaviour (auto-place, no copper)

## ADDED Requirements

### Requirement: PCB emitter emits copper geometry from Layout
When a `Layout` is supplied, the PCB emitter SHALL emit `(segment …)` nodes for layout tracks, `(via …)` nodes for layout vias, and `(zone …)` nodes for layout zones. Each copper element SHALL carry its net's KiCad net code, layer, and a deterministic UUID seeded from net, layer, and ordinal. Nets present in `LayoutReport.unrouted` SHALL remain as ratsnest airwires (no copper), and their absence SHALL NOT fail emission.

#### Scenario: Tracks become segments
- **WHEN** the layout contains a track on net `V3V3`, layer F.Cu, width 0.25 mm
- **THEN** the `.kicad_pcb` contains `(segment (start …) (end …) (width 0.25) (layer "F.Cu") (net <code for V3V3>))` entries covering the track path

#### Scenario: Plane net becomes a zone
- **WHEN** the layout contains a zone for GND on layer 31 covering the board outline
- **THEN** the `.kicad_pcb` contains a `(zone (net <code for GND>) (layer "B.Cu") …)` whose polygon matches the board outline

#### Scenario: Unrouted net emits no copper
- **WHEN** a net is listed in `LayoutReport.unrouted`
- **THEN** the `.kicad_pcb` contains no segments or vias for that net
- **AND** emission succeeds
