## ADDED Requirements

### Requirement: Pad is the single pad-geometry model
A `Pad` struct SHALL be defined in the core crate as the only representation of footprint pad geometry in the workspace. It SHALL carry: `number`, `pos`, `rotation`, `width`, `height`, `pad_type`, `shape`, `roundrect_rratio`, `solder_mask_margin`, `layers`, and `drill`. The structs `MechanicalPad` (core), `fp_geom::PadGeom` (backend), and `MechanicalDef`/`ThermalViaDef` (codegen) SHALL be removed and replaced by `Pad` (and `ThermalVia` for via-in-pad data). No other struct in the workspace SHALL re-declare pad geometry fields.

#### Scenario: Mechanical pads use the same type as electrical pads
- **WHEN** a component declares a mounting hole or paste aperture
- **THEN** it is stored as a `Pad` in the component's mechanical pad list, with no separate mechanical-pad struct

#### Scenario: Codegen pin definition embeds Pad
- **WHEN** a parts TOML describes a pin with pad geometry
- **THEN** the geometry deserialises directly into a core `Pad` — no field-by-field mapping code exists between the TOML schema and the core type

### Requirement: Pad type and shape are typed enums
`Pad` SHALL use a `PadType` enum (`Smd`, `ThruHole`, `NpThruHole`, `Connect`) and a `PadShape` enum (`Rect`, `RoundRect`, `Circle`, `Oval`) instead of free-form strings. KiCad string representations SHALL be parsed/serialised at the format boundary (parsers and emitters) only. An unrecognised string SHALL produce a parse error naming the offending value.

#### Scenario: Parser converts KiCad strings at the boundary
- **WHEN** `fp_parser` reads `(pad "1" smd roundrect ...)`
- **THEN** the resulting `Pad` has `pad_type == PadType::Smd` and `shape == PadShape::RoundRect`

#### Scenario: Unknown pad type is a parse error
- **WHEN** a footprint contains `(pad "1" quantum rect ...)`
- **THEN** parsing fails with an error naming `"quantum"`

### Requirement: SymPin carries schematic pin graphics
A `SymPin` struct SHALL be defined in the core crate carrying schematic symbol pin graphics: `pos`, `rotation`, and `length`. A `Pin` SHALL hold `symbol: Option<SymPin>`. The `length` field SHALL represent the schematic pin stub length only; it SHALL NOT be used as a source of pad dimensions.

#### Scenario: Symbol graphics do not leak into pad geometry
- **WHEN** a pin has `symbol.length = 2.54` and no pad width
- **THEN** pad resolution applies the documented default pad size — the symbol stub length is never substituted for pad width

### Requirement: Pin is electrical identity plus optional physical data
A `Pin` SHALL carry electrical identity and specification (`id`, `name`, `number`, `role`, `power_spec`, `decouple`, `sig_spec`, `thermal_vias`, `nc`) plus `pad: Option<Pad>` and `symbol: Option<SymPin>`. It SHALL NOT carry bare physical fields (`pos`, `rotation`, `length`, `width`, `height`, `pad_type`, `pad_shape`, `roundrect_rratio`, `solder_mask_margin`, `layers`, `drill`) of its own.

#### Scenario: Pin with pad geometry
- **WHEN** a pin is constructed with `.pad(Pad { .. })`
- **THEN** `pin.pad()` returns the pad and the pin exposes no standalone `pos()`/`width()` accessors

### Requirement: resolve_pad owns all pad defaulting rules
A single `resolve_pad(pin, index) -> Pad` function SHALL resolve a pin's pad to fully-populated geometry for emission, and SHALL be the only place the following defaults are applied: auto-row position fallback (2.54 mm pitch, pad 1 at origin), pad-type default (explicit `pad_type` wins; otherwise SMD iff the pin has an explicit position, else through-hole), width/height fallback, layer defaults by pad type, drill default for through-hole pads, shape default (pad 1 rectangular, others circular, for auto through-hole rows), and anchor normalisation (SMD footprints recentred on the pad bounding box; through-hole footprints anchored on pad 1). Both the CLI `generate` pipeline and board emission SHALL use this function.

#### Scenario: Both pipelines produce identical pads
- **WHEN** the same part is emitted as a standalone `.kicad_mod` via `generate footprint` and embedded in a `.kicad_pcb` via board emission
- **THEN** the pad `(at ...)` `(size ...)` `(layers ...)` and `(drill ...)` values are identical

#### Scenario: Missing geometry falls back to auto-row
- **WHEN** a pin has no pad data
- **THEN** `resolve_pad` places a through-hole pad at `index * 2.54 mm` on the X axis with the documented default size, drill, and layers
