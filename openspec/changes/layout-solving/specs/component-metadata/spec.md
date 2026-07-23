## MODIFIED Requirements

### Requirement: Component trait collapses to cohesive methods
The `Component` trait SHALL expose `pins()`, `meta()`, `mechanical()`, `constraints()`, and `layout_constraints()` — and SHALL NOT expose per-field getters (`symbol()`, `footprint()`, `datasheet()`, `description()`, `model_3d()`, `model_3d_data()`, `model_3d_rotation()`, `model_3d_offset()`). `meta()` SHALL have a default implementation returning an empty `ComponentMeta`. `layout_constraints()` SHALL have a default implementation returning an empty vector.

#### Scenario: Minimal component compiles with one method
- **WHEN** a hand-written component implements only `pins()`
- **THEN** it satisfies the `Component` trait and its metadata and layout constraints are treated as empty

### Requirement: CompiledComponent embeds ComponentMeta
`CompiledComponent` SHALL consist of `refdes`, `meta: ComponentMeta`, `pins`, `mechanical: Vec<Pad>`, `constraints`, and `layout: Vec<LayoutConstraint>` — no flattened per-field metadata. A single `CompiledComponent::from_component(refdes, &dyn Component)` constructor SHALL be the only trait→compiled conversion, used by both board lowering and decoupling-capacitor synthesis. The constructor SHALL carry both `constraints()` and `layout_constraints()` from the trait into the compiled component.

#### Scenario: Synthesised and lowered components share one constructor
- **WHEN** the compiler lowers board components and synthesises a decoupling capacitor
- **THEN** both are produced by `CompiledComponent::from_component` with no duplicated field-copying code

#### Scenario: Layout constraints flow through the constructor
- **WHEN** a component's `layout_constraints()` returns a `PlaceAt` directive
- **THEN** the `CompiledComponent` produced by `from_component` carries that directive in its `layout` field
