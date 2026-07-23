## MODIFIED Requirements

### Requirement: CompiledBoard is the immutable verified artifact
The `CompiledBoard` struct SHALL be immutable and contain all resolved data: components (original + synthesised) with deterministic refdes, inferred nets with properties, resolved connections, and all constraints — both electrical (`constraints`) and physical (`layout` on components, nets, and the board itself). During lowering, a `LayoutConstraint::NetClass` directive on a net SHALL resolve into that net's `Net.class` (explicit directive wins; absent directives leave the default class).

#### Scenario: CompiledBoard carries synthesised components
- **WHEN** a board with a component requiring decoupling is compiled
- **THEN** the `CompiledBoard` contains both the original components and the synthesised decoupling capacitors with deterministic refdes

#### Scenario: NetClass directive resolves into the net class
- **WHEN** a net carries `LayoutConstraint::NetClass { min_width: 0.5.mm(), clearance: 0.2.mm() }`
- **THEN** the compiled net's `class` field has `min_width == Some(0.5 mm)` and `clearance == Some(0.2 mm)`

### Requirement: Backend trait provides emission interface
A `Backend` trait SHALL define `fn emit(&self, output_dir: &str, board: &CompiledBoard) -> Result<(), Self::Error>`. It SHALL also define a provided method `fn emit_with_layout(&self, output_dir: &str, board: &CompiledBoard, layout: &Layout) -> Result<(), Self::Error>` whose default implementation ignores the layout and delegates to `emit()`. Backends (KiCad, future SPICE, etc.) SHALL implement this trait. The developer's `main.rs` SHALL construct a backend and call `emit()` (or `emit_with_layout()`) with a `CompiledBoard`.

#### Scenario: KiCad backend emits to filesystem
- **WHEN** `KiCad::new().emit("output/", &compiled_board)` is called
- **THEN** KiCad project files (`.kicad_sch`, `.kicad_pcb`, `.kicad_pro`, `.net`) are written to the `output/` directory

#### Scenario: emit_with_layout defaults to emit
- **WHEN** a backend implements only `emit()` and `emit_with_layout()` is called
- **THEN** the call delegates to `emit()` and the layout is ignored

#### Scenario: main.rs is the sole source of truth
- **WHEN** a project's `main.rs` builds a board, compiles it, optionally solves a layout, and calls `backend.emit()` or `backend.emit_with_layout()`
- **THEN** `cargo run` produces the complete backend output with no intermediary files or CLI steps
