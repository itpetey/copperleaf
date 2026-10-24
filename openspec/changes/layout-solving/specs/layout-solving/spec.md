## ADDED Requirements

### Requirement: Layout is the core-owned physical IR
A `Layout` struct SHALL be defined in the core crate carrying `placements: Vec<Placement>`, `tracks: Vec<Track>`, `vias: Vec<Via>`, and `zones: Vec<Zone>`. Placements SHALL reference components by index into `CompiledBoard.components`; tracks, vias, and zones SHALL reference nets by `NetIdx`. The IR SHALL derive `Clone`, `Debug`, and serde, SHALL preserve construction order, and SHALL NOT expose any external solver type.

#### Scenario: Layout references are stable identities
- **WHEN** a `Track` references `NetIdx(3)`
- **THEN** that index resolves through `CompiledBoard::net(NetIdx(3))` to the same net

#### Scenario: No external solver types leak into core
- **WHEN** the public API of `crates/core` is inspected
- **THEN** no item names an external autorouter or solver type

### Requirement: copperleaf-layout crate solves placement
A new `crates/layout` crate (`copperleaf-layout`) SHALL expose `solve(board: &CompiledBoard, options: &SolveOptions) -> Result<LayoutReport, LayoutError>`. `LayoutReport` SHALL contain the solved `Layout` and `diagnostics: Vec<Diagnostic>`. The crate SHALL perform deterministic placement and plane-zone generation only; it SHALL NOT route and SHALL NOT depend on an autorouter.

#### Scenario: Solve returns placements and zones
- **WHEN** `solve()` is called on a compiled board
- **THEN** the returned `LayoutReport` contains a `Layout` with one placement for every component
- **AND** a `Zone` for every `Plane` net

### Requirement: Placement honours PlaceAt and reports unsupported directives
The solver SHALL treat `PlaceAt` placements as fixed. Directives it does not implement (`PlaceNear`, `SameSide`, `Keepout`, and routing-only constraints) SHALL produce a `Warning`-severity `LAYOUT:UNSUPPORTED_DIRECTIVE` diagnostic instead of being silently ignored.

#### Scenario: Fixed placement is respected
- **WHEN** a component carries `PlaceAt { pos: (10.0, 40.0), rotation: 90.0, side: Front }`
- **THEN** the solved `Placement` for that component has exactly that position, rotation, and side

#### Scenario: Unsupported directive produces a warning
- **WHEN** a component carries `PlaceNear`
- **THEN** `solve()` still returns a `LayoutReport`
- **AND** the directive appears in `diagnostics` with code `LAYOUT:UNSUPPORTED_DIRECTIVE`

### Requirement: Plane nets become zones
A net carrying `LayoutConstraint::Plane { layer }` SHALL produce a `Zone` for that net on the given copper ordinal covering the board outline. No tracks or vias are generated for that net.

#### Scenario: Ground plane produces a zone
- **WHEN** the GND net carries `Plane { layer: CopperLayer(1) }` on a two-layer board
- **THEN** the solved `Layout` contains a `Zone` for GND on ordinal 1
- **AND** no GND tracks or vias are produced

### Requirement: Internal DRC validates solved geometry
The solver SHALL run a design-rule check over the solved `Layout` before returning: track/via clearances against each net's resolved `NetClass` clearance, track widths against `min_width`, and `Creepage` minima. Violations SHALL appear in `LayoutReport.diagnostics` with codes prefixed `LAYOUT:` and SHALL NOT fail the solve unless the geometry is self-intersecting.

#### Scenario: Clearance violation is reported
- **WHEN** the layout contains two tracks closer than their nets' clearance rule
- **THEN** the report contains a `Warning` diagnostic with a `LAYOUT:`-prefixed code naming the nets involved

### Requirement: Solving is deterministic for a fixed seed
`SolveOptions` SHALL carry an explicit `seed: u64`. Two `solve()` calls with the same `CompiledBoard` and the same seed SHALL produce identical placement output, byte-stable across processes.

#### Scenario: Same seed, same placements
- **WHEN** `solve()` runs twice in separate processes with `seed = 42` on the same compiled board
- **THEN** both placement lists are byte-identical
