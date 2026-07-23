## ADDED Requirements

### Requirement: Layout solving is deterministic for a fixed seed
Layout solving SHALL be deterministic: two `solve()` invocations in separate processes with the same `CompiledBoard` and the same `SolveOptions.seed` SHALL produce equal `LayoutReport` values. No layout code SHALL use unseeded randomness, wall-clock time, or hash-iteration order in output paths.

#### Scenario: Same seed produces identical layout
- **WHEN** `solve()` runs twice in separate processes with `seed = 42` on the same compiled board
- **THEN** both `LayoutReport` values are equal, including all placement coordinates

#### Scenario: Layout emission remains byte-identical
- **WHEN** the same `CompiledBoard` and the same solved `Layout` are emitted twice via `emit_with_layout()`
- **THEN** both `.kicad_pcb` outputs are byte-for-byte identical, including copper element UUIDs
