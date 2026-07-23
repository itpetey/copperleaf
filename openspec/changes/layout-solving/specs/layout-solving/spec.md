## ADDED Requirements

### Requirement: Layout is the core-owned physical IR
A `Layout` struct SHALL be defined in the core crate carrying `placements: Vec<Placement>`, `tracks: Vec<Track>`, `vias: Vec<Via>`, and `zones: Vec<Zone>`. Placements SHALL reference components by index into `CompiledBoard.components`; tracks, vias, and zones SHALL reference nets by `NetIdx`. The IR SHALL derive `Clone`, `Debug`, and serde, SHALL preserve construction order, and SHALL NOT expose any type from the `topola` crate.

#### Scenario: Layout references are stable identities
- **WHEN** a `Track` references `NetIdx(3)`
- **THEN** that index resolves through `CompiledBoard::net(NetIdx(3))` to the same net the solver routed

#### Scenario: No topola types leak into core
- **WHEN** the public API of `crates/core` is inspected
- **THEN** no item names a type from the `topola` crate

### Requirement: copperleaf-layout crate solves placement and routing
A new `crates/layout` crate (`copperleaf-layout`) SHALL expose `solve(board: &CompiledBoard, options: &SolveOptions) -> Result<LayoutReport, LayoutError>`. `LayoutReport` SHALL contain the solved `Layout`, an `unrouted: Vec<NetIdx>` list, and `diagnostics: Vec<Diagnostic>`. The crate SHALL embed the `topola` crate for placement and routing, with all topola usage confined to a single adapter module; no other module or workspace crate SHALL depend on `topola`.

#### Scenario: Solve returns a complete report
- **WHEN** `solve()` is called on a compiled board with routeable nets
- **THEN** the returned `LayoutReport` contains a `Layout` with placements for every component and tracks/vias for routed nets

#### Scenario: Topola dependency is contained
- **WHEN** the workspace dependency graph is inspected
- **THEN** only `copperleaf-layout` depends on `topola`

### Requirement: Placement honours placement directives
The solver SHALL treat `PlaceAt` placements as fixed. The solver SHALL honour `PlaceNear` (within `max_radius` of the target), `SameSide`, and `Keepout` directives, or emit a `Warning`-severity diagnostic for each directive it cannot satisfy.

#### Scenario: Fixed placement is respected
- **WHEN** a component carries `PlaceAt { pos: (10.0, 40.0), rotation: 90.0, side: Front }`
- **THEN** the solved `Placement` for that component has exactly that position, rotation, and side

#### Scenario: Unsatisfiable directive produces a warning, not a failure
- **WHEN** two components carry `PlaceNear` directives that cannot both be satisfied on the available board area
- **THEN** `solve()` still returns a `LayoutReport`
- **AND** the unsatisfied directive appears in `diagnostics` with severity `Warning`

### Requirement: Unrouted nets are reported, never fatal
The solver SHALL attempt to route every non-plane net. Nets it cannot route SHALL appear in `LayoutReport.unrouted`; `solve()` SHALL NOT fail because nets are unrouted. Unrouted nets SHALL remain visible as ratsnest airwires when emitted.

#### Scenario: Partial routing succeeds
- **WHEN** the router cannot complete two of ten nets
- **THEN** `solve()` returns `Ok` with eight nets routed in `Layout` and two entries in `unrouted`

### Requirement: Plane nets become zones
A net carrying `LayoutConstraint::Plane { layer }` SHALL be excluded from routing. The solved `Layout` SHALL contain a `Zone` for that net on the given layer covering the board outline. Plane-net pads SHALL rely on the zone pour for connectivity (filled by KiCad on open or refill).

#### Scenario: Ground plane produces a zone, not tracks
- **WHEN** the GND net carries `Plane { layer: 31 }`
- **THEN** the solved `Layout` contains a `Zone` for GND on layer 31
- **AND** no GND tracks or vias are produced

### Requirement: Internal DRC validates solved geometry
The solver SHALL run a design-rule check over the solved `Layout` before returning: track/via clearances against each net's resolved `NetClass` clearance, track widths against `min_width`, and `Creepage` minima. Violations SHALL appear in `LayoutReport.diagnostics` with codes prefixed `LAYOUT:` and SHALL NOT fail the solve (severity `Warning` unless the geometry is self-intersecting, which is `Error`).

#### Scenario: Clearance violation is reported
- **WHEN** the router produces two tracks closer than their nets' clearance rule
- **THEN** the report contains a `Warning` diagnostic with a `LAYOUT:`-prefixed code naming the nets involved

### Requirement: Solving is deterministic for a fixed seed
`SolveOptions` SHALL carry an explicit `seed: u64`. Two `solve()` calls with the same `CompiledBoard` and the same seed SHALL produce identical `LayoutReport` values. Placement output SHALL be byte-stable across processes.

#### Scenario: Same seed, same layout
- **WHEN** `solve()` runs twice in separate processes with `seed = 42` on the same compiled board
- **THEN** both `LayoutReport` values are equal

#### Scenario: Determinism is tested honestly
- **WHEN** routing geometry cannot yet be made byte-stable (pending Topola seed control)
- **THEN** golden tests assert structural invariants — every net routed or listed in `unrouted`, and no DRC diagnostics — instead of byte-identical copper
