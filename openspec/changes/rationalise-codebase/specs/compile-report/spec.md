## MODIFIED Requirements

### Requirement: CompileSummary provides auditable compiler decisions
`CompileSummary` SHALL contain `nets: Vec<NetInfo>` (each with net name, kind, and pin count), `pin_count: usize`, and `component_count: usize`. Synthesised decoupling capacitors SHALL be auditable via the compiled board's component list (deterministic `C1…` refdes) and reported via a `DECOUPLE:SUMMARY` info diagnostic. The developer SHALL be able to inspect the summary to audit what the compiler did.

#### Scenario: Summary lists inferred nets
- **WHEN** a board with three connections is compiled
- **THEN** `summary.nets` contains one entry per inferred net with its name, kind, and pin count

#### Scenario: Synthesised caps are auditable
- **WHEN** a board with a decoupling constraint is compiled
- **THEN** the synthesised capacitors appear in `CompiledBoard.components` with deterministic refdes
- **AND** a `DECOUPLE:SUMMARY` info diagnostic reports how many were placed

## REMOVED Requirements

### Requirement: SynthCap records synthesised capacitor details
**Reason**: Never implemented — the spec described a `SynthCap` struct and `CompileSummary.caps_synthesised` field that the compiler does not produce. Synthesised capacitors are audited via the compiled board's component list and the `DECOUPLE:SUMMARY` diagnostic instead.
**Migration**: Consumers inspect `CompiledBoard.components` for synthesised capacitors (refdes `C1…`) and the `DECOUPLE:SUMMARY` diagnostic for counts.
