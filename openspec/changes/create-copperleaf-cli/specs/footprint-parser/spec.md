## ADDED Requirements

### Requirement: Footprint parser parses KiCad footprint files
A `fp_parser` module SHALL be available in the `copperleaf-backend-kicad` crate. It SHALL parse `.kicad_mod` files via the existing `sexpr::parse` function and extract `PadDef` structs with pad numbers, positions, rotations, and dimensions. It SHALL NOT be used during `Board::compile()` or `Backend::emit()`.

#### Scenario: parse_footprint extracts pad definitions
- **WHEN** a `.kicad_mod` file is parsed by `fp_parser::parse_footprint`
- **THEN** `PadDef` structs are returned with pad numbers, positions, rotations, widths, heights, and pad types

#### Scenario: Footprint with no pads returns empty list
- **WHEN** a `.kicad_mod` file with no `(pad ...)` nodes is parsed
- **THEN** `parse_footprint` returns an empty `Vec<PadDef>`

### Requirement: PadDef carries physical pad data
A `PadDef` struct SHALL be publicly re-exported from `copperleaf_backend_kicad::lib` with fields: `number: String`, `pos: (f64, f64)`, `rotation: f64`, `width: f64`, `height: f64`, `pad_type: String`. The `number` field SHALL match the `number` field of `sym_parser::PinDef` so that symbol and footprint data can be merged by pin number.

#### Scenario: PadDef number matches symbol PinDef number
- **WHEN** a symbol pin has `number = "1"` and a footprint pad has `number = "1"`
- **THEN** the two can be matched by pin number during `update --footprint`

### Requirement: parse_footprint accepts .kicad_mod and .pretty directories
`parse_footprint` SHALL accept a path to a single `.kicad_mod` file. A separate `parse_footprint_lib` function SHALL accept a `.pretty` directory and return a list of named footprints.

#### Scenario: Single .kicad_mod file parsed
- **WHEN** `parse_footprint("RP2354A.kicad_mod")` is called
- **THEN** a `Vec<PadDef>` is returned for the single footprint

#### Scenario: .pretty directory parsed
- **WHEN** `parse_footprint_lib("Package_QFP.pretty/")` is called
- **THEN** a `Vec<(String, Vec<PadDef>)>` is returned, one entry per `.kicad_mod` file in the directory