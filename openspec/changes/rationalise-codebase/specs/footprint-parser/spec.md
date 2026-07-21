## MODIFIED Requirements

### Requirement: PadDef carries physical pad data
A `PadDef` struct SHALL be publicly re-exported from `copperleaf_backend_kicad` as the KiCad-format parse result for one pad, with fields: `number`, `pos`, `rotation`, `width`, `height`, `pad_type`, `shape`, `roundrect_rratio`, `solder_mask_margin`, `layers`, and `drill`. The `number` field SHALL match the `number` field of `sym_parser::PinDef` so that symbol and footprint data can be merged by pin number. `PadDef` SHALL provide bidirectional conversion to the core `Pad` type (`to_pad`/`from_pad`); this conversion SHALL be the only place KiCad pad type/shape strings are translated to the typed core enums.

#### Scenario: PadDef number matches symbol PinDef number
- **WHEN** a symbol pin has `number = "1"` and a footprint pad has `number = "1"`
- **THEN** the two can be matched by pin number during `update --footprint`

#### Scenario: PadDef converts to a typed core Pad
- **WHEN** a parsed `PadDef` with `pad_type = "smd"` and `shape = "roundrect"` is converted
- **THEN** the resulting core `Pad` has `pad_type == PadType::Smd` and `shape == PadShape::RoundRect`
