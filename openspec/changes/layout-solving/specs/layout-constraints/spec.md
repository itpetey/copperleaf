## ADDED Requirements

### Requirement: LayoutConstraint enum carries physical directives
A `LayoutConstraint` enum SHALL be defined in the core crate as the only representation of physical layout directives. It SHALL include variants: `NetClass { min_width, clearance }`, `Creepage { min, voltage }` (both moved from the `Constraint` enum), `PlaceAt { pos, rotation, side }`, `PlaceNear { target, max_radius }`, `SameSide { group }`, `Keepout { region, layers }`, and `Plane { layer }`. The `Constraint` enum SHALL NOT contain these variants. Supporting vocabulary types `BoardSide` (front/back), `Region` (rect/circle), and `LayerSet` SHALL be defined once in core and shared by the variants.

#### Scenario: NetClass moves out of Constraint
- **WHEN** a component author writes `LayoutConstraint::NetClass { min_width: 0.5.mm(), clearance: 0.2.mm() }`
- **THEN** it type-checks as a `LayoutConstraint`
- **AND** `Constraint::NetClass` no longer exists

#### Scenario: Placement directive is expressible
- **WHEN** a board author constrains a connector with `PlaceAt { pos: (10.0, 40.0), rotation: 90.0, side: BoardSide::Front }`
- **THEN** the directive is stored on the component's layout constraint list for the solver to honour as a fixed placement

### Requirement: Layout constraints attach at component, net, and board level
`LayoutConstraint` values SHALL attach at three levels, mirroring the existing `constraints` fields: `CompiledComponent.layout`, `Net.layout`, and `CompiledBoard.layout`. The `Component` trait SHALL expose `fn layout_constraints(&self) -> Vec<LayoutConstraint>` with a default implementation returning an empty vector. `CompiledComponent::from_component` SHALL carry the trait's layout constraints into the compiled component.

#### Scenario: Component layout constraints survive compilation
- **WHEN** a component's `layout_constraints()` returns `vec![LayoutConstraint::PlaceAt { .. }]`
- **THEN** the compiled board's component entry contains that directive in its `layout` field

#### Scenario: Default is empty
- **WHEN** a component implements only `pins()`
- **THEN** its compiled `layout` field is empty

### Requirement: Board builder API for layout directives
The `Board` builder SHALL provide typed methods for board-level directives: `place_at(handle, pos, rotation, side)`, `place_near(handle, target, max_radius)`, `keepout(region, layers)`, and `assign_plane(net_handle, layer)`. These SHALL validate their component/net handles the same way `connect()` does and store the directives on the corresponding component, net, or board constraint lists during lowering.

#### Scenario: Plane assignment reaches the net
- **WHEN** `board.assign_plane(gnd_handle, 31)` is called for the ground net on B.Cu
- **THEN** the compiled `Net` for GND carries `LayoutConstraint::Plane { layer: 31 }` in its `layout` field

#### Scenario: Invalid handle is rejected
- **WHEN** `place_at` is called with a component handle that does not exist
- **THEN** a `CompileError` diagnostic is produced, consistent with `connect()` validation

### Requirement: Decoupling synthesis attaches PlaceNear automatically
When decoupling-capacitor synthesis creates a capacitor for a `PowerIn` pin, it SHALL attach `LayoutConstraint::PlaceNear { target: <the pin's component>, max_radius }` to the synthesised component's `layout` list. The default radius SHALL be a named constant (5 mm) overridable via solver options.

#### Scenario: Synthesised cap is constrained near its target
- **WHEN** a component declares `Decoupling { values: [100.0.nf()], per_pin: true }` and compilation synthesises a capacitor
- **THEN** the synthesised component carries `PlaceNear` targeting the decoupled component with the default radius

### Requirement: Parts TOML schema gains a layout section
The parts TOML `Manifest` SHALL express physical directives in a `[layout]` table (serde-mapping onto core `LayoutConstraint` values), including the `net_class` and `creepage` keys moved out of the electrical constraints section. Codegen SHALL emit these as the component's `layout_constraints()` implementation. TOML manifests using the old locations SHALL fail deserialisation with an error naming the moved key.

#### Scenario: TOML layout section generates layout_constraints()
- **WHEN** a parts TOML declares `[layout] net_class = { min_width = "0.5mm", clearance = "0.2mm" }`
- **THEN** the generated component returns that directive from `layout_constraints()`

#### Scenario: Old key location is rejected
- **WHEN** a parts TOML declares `net_class` in the electrical constraints section
- **THEN** manifest deserialisation fails with an error naming `net_class` and its new `[layout]` location
