## Why

Copperleaf compiles a typed, constraint-checked design into a `CompiledBoard`, but physical layout is where the pipeline stops: the KiCad PCB emitter packs footprints into naive rows (`auto_place`) and emits no copper at all — no tracks, no vias, no zones. Every board project must finish placement and routing by hand in KiCad, so the constraints the library already declares (net classes, impedance, length matching, creepage) have no physical enforcement and no feedback.

Worse, the `Constraint` enum conflates two vocabularies. Electrical/behavioural intent (`Decoupling`, `MaxJunction`, `ResonanceIndex`) is consumed by ERC and synthesis, while physical directives (`NetClass`, `Creepage`) are declared but **nothing consumes them** — `Net.class` is hard-coded to `NetClass::default()` during lowering, so the power-net-class emission path in the KiCad backend is dead code. A solver needs these directives as an explicit, well-typed input; today they are decoration.

## What Changes

- **Phase 1 — Constraint split. BREAKING.** The `Constraint` enum is narrowed to electrical/behavioural intent: `Decoupling`, `ResonanceIndex`, `MaxJunction`, `Impedance`, `LengthMatch`, `ReturnPath`. A new `LayoutConstraint` enum in core carries physical directives: `NetClass { min_width, clearance }` and `Creepage { min, voltage }` move across; new variants `PlaceAt`, `PlaceNear`, `SameSide`, `Keepout`, and `Plane` give the solver explicit placement/routing input. `LayoutConstraint` attaches at all three levels — component, net, board — alongside the existing `constraints` fields, and the `Component` trait gains `layout_constraints()` (default empty). The parts TOML schema and codegen move `net_class`/`creepage` into a `[layout]` section. Decoupling-capacitor synthesis automatically attaches `PlaceNear` linking each synthesised cap to its target power pin.
- **Phase 2 — Layout IR and backend consumption.** A new `Layout` struct in core (`placements`, `tracks`, `vias`, `zones`, referencing components by index and nets by `NetIdx`) is the backend-facing physical artifact. The `Backend` trait gains a provided `emit_with_layout()` method (default: ignore layout, behave as today). The KiCad PCB emitter consumes `Layout` when present: real placements and rotations replace `auto_place`, and `(segment …)`, `(via …)`, and `(zone …)` nodes are emitted. Net-class resolution is repaired: `LayoutConstraint::NetClass` on a net resolves into `Net.class` during lowering, reviving the dead net-class emission path.
- **Phase 3 — placement-only `copperleaf-layout` crate.** A new `crates/layout` crate exposes `solve(board, options) -> Result<LayoutReport, LayoutError>`. It performs deterministic placement, DRC, and plane-zone generation only. There is **no autorouter**: routing, unrouted-net reporting, Topola integration, and routed-copper claims are explicitly deferred to a separate change. Placement directives that the solver does not yet honour (`PlaceNear`, `SameSide`, `Keepout`) are reported as unsupported-directive diagnostics rather than silently ignored.
- **Determinism.** `SolveOptions` carries an explicit `seed: u64`. For a fixed seed and fixed `CompiledBoard`, placement is deterministic and has byte-exact goldens.

## Capabilities

### New Capabilities
- `layout-constraints`: The `LayoutConstraint` enum — variants, semantics, and attachment at component/net/board level; the `Component::layout_constraints()` trait method; `Board` builder APIs for placement directives; decoupling-synthesis `PlaceNear` auto-attachment; parts TOML `[layout]` schema.
- `layout-solving`: The `Layout` IR in core; the `copperleaf-layout` crate and its placement-only `solve()` pipeline (translate → place → DRC → report); determinism requirements; and zone generation for `Plane` nets. Autorouting is out of scope.

### Modified Capabilities
- `erc-and-synthesis`: The `Constraint` enum loses `NetClass` and `Creepage` (moved to `LayoutConstraint`); the documented variant list is updated to the electrical-only set.
- `component-metadata`: The `Component` trait gains `layout_constraints()`; `CompiledComponent` gains a `layout` field.
- `board-compile-pipeline`: `CompiledBoard` and `Net` gain `layout` fields; the `Backend` trait gains the provided `emit_with_layout()` method; lowering resolves `LayoutConstraint::NetClass` into `Net.class`.
- `kicad-backend`: The PCB emitter consumes `Layout` (placements, segments, vias, zones) when provided via `emit_with_layout()`, falling back to `auto_place` + ratsnest otherwise; net-class emission consumes the now-populated `Net.class`.
- `deterministic-ids`: Determinism guarantee extended to layout solving for a fixed seed.

## Impact

- **`crates/core`**: `LayoutConstraint` enum and `Layout` IR (`placements`/`tracks`/`vias`/`zones`); narrowed `Constraint`; `Component::layout_constraints()`; `Board` layout-directive builder methods; `Backend::emit_with_layout()` provided method; `layout` fields on `CompiledBoard`/`CompiledComponent`/`Net`; new `BoardSide`/`Region`/`LayerSet` vocabulary types.
- **`crates/compile`**: Net-class resolution into `Net.class`; `PlaceNear` auto-attachment during decoupling synthesis; `layout` fields populated during lowering.
- **`crates/layout`** (new): placement-only `solve()` pipeline, internal DRC, `SolveOptions`/`LayoutReport`/`LayoutError`. No autorouter and no external solver dependency.
- **`crates/backend-kicad`**: `emit_with_layout()` implementation; segment/via/zone emission; placements from `Layout`; `auto_place` retained as fallback only; revived net-class emission.
- **`crates/part-codegen`, `crates/cli`, `parts/*`**: TOML `[layout]` section; codegen emits `layout_constraints()`; manifests regenerated (breaking schema change for `net_class`/`creepage` keys).
- **Downstream board projects**: `Constraint::NetClass`/`Constraint::Creepage` references migrate to `LayoutConstraint`; `main.rs` gains two lines (`solve()` + `emit_with_layout()`) to opt into placement-aware output.
- **Golden tests**: Phase 1 regenerates codegen goldens (enum rename only); Phase 2 goldens byte-identical when no layout is passed; Phase 3 adds byte-exact placement goldens and zone generation; there are no routing goldens because routing is not implemented.
- **Risk posture**: Placement quality is modest and some directives are not yet honoured. The change states those limitations explicitly and reports unsupported directives as diagnostics; routed fabrication output requires a future change.
