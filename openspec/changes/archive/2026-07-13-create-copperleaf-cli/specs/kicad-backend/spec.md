## ADDED Requirements

### Requirement: Footprint parser is brought across for CLI use
A `fp_parser` module SHALL be available in the KiCad backend crate for use by the parts-creation CLI. It SHALL parse `.kicad_mod` footprint files and extract `PadDef` structs with pad numbers, positions, rotations, and dimensions. It SHALL NOT be used during `Board::compile()` or `Backend::emit()`.

#### Scenario: Footprint parser extracts pad definitions
- **WHEN** a `.kicad_mod` file is parsed by `fp_parser`
- **THEN** `PadDef` structs are returned with pad numbers, positions, rotations, widths, heights, and pad types