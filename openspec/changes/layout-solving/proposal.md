## Why

Copperleaf compiles a typed, constraint-checked design into a `CompiledBoard`, but physical layout is where the pipeline stops: the KiCad PCB emitter packs footprints into naive rows (`auto_place`) and emits no copper at all — no tracks, no vias, no zones. Every board project must finish placement and routing by hand in KiCad, so the constraints the library already declares (net classes, impedance, length matching, creepage) have no physical enforcement and no feedback.

Worse, the `Constraint` enum conflates two vocabularies. Electrical/behavioural intent (`Decoupling`, `MaxJunction`, `ResonanceIndex`) is consumed by ERC and synthesis, while physical directives (`NetClass`, `Creepage`) are declared but **nothing consumes them** — `Net.class` is hard-coded to `NetClass::default()` during lowering, so the power-net-class emission path in the KiCad backend is dead code. A solver needs these directives as an explicit, well-typed input; today they are decoration.

## What Changes

- **Phase 1 — Constraint split. BREAKING.** The `Constraint` enum is narrowed to electrical/behavioural intent: `Decoupling`, `ResonanceIndex`, `MaxJunction`, `Impedance`, `LengthMatch`, `ReturnPath`. A new `LayoutConstraint` enum in core carries physical directives: `NetClass { min_width, clearance }` and `Creepage { min, voltage }` move across; new variants `PlaceAt`, `PlaceNear`, `SameSide`, `Keepout`, and `Plane` give the solver explicit placement/routing input. `LayoutConstraint` attaches at all three levels — component, net, board — alongside the existing `constraints` fields, and the `Component` trait gains `layout_constraints()` (default empty). The parts TOML schema and codegen move `net_class`/`creepage` into a `[layout]` section. Decoupling-capacitor synthesis automatically attaches `PlaceNear` linking each synthesised cap to its target power pin.
- **Phase 2 — Layout IR and backend consumption.** A new `Layout` struct in core (`placements`, `tracks`, `vias`, `zones`, referencing components by index and nets by `NetIdx`) is the backend-facing physical artifact. The `Backend` trait gains a provided `emit_with_layout()` method (default: ignore layout, behave as today). The KiCad PCB emitter consumes `Layout` when present: real placements and rotations replace `auto_place`, and `(segment …)`, `(via …)`, and `(zone …)` nodes are emitted. Net-class resolution is repaired: `LayoutConstraint::NetClass` on a net resolves into `Net.class` during lowering, reviving the dead net-class emission path.
- **Phase 3 — `copperleaf-layout` crate with embedded Topola.** A new `crates/layout` crate exposes `solve(board, options) -> Result<LayoutReport, LayoutError>`, embedding [Topola](https://codeberg.org/topola/topola) (`topola` 0.1.0, MIT OR Apache-2.0) rather than writing a router from scratch. A single `topola_adapter` module translates `CompiledBoard` geometry into Topola's board model and back into the `Layout` IR — it is the only module depending on the `topola` crate, containing all 0.x API churn. Placement runs through Topola's autoplacer honouring `PlaceAt`/`PlaceNear`/`SameSide`; routing runs through Topola's autorouter honouring net-class widths/clearances; nets constrained `Plane` are not routed — they become KiCad zones poured by KiCad on open. Unrouteable nets are reported in `LayoutReport.unrouted` and emitted as ratsnest airwires, never a hard failure. An internal DRC pass (clearance/width/creepage over the solved geometry) validates Topola's output before emission.
- **Determinism.** `SolveOptions` carries an explicit `seed: u64`. For a fixed seed and fixed `CompiledBoard`, `solve()` SHALL be deterministic. Placement goldens are byte-exact; routing goldens assert structural invariants (connectivity completeness, DRC-clean) until Topola's seed control is confirmed sufficient for byte-stable copper (see design D5).

## Capabilities

### New Capabilities
- `layout-constraints`: The `LayoutConstraint` enum — variants, semantics, and attachment at component/net/board level; the `Component::layout_constraints()` trait method; `Board` builder APIs for placement directives; decoupling-synthesis `PlaceNear` auto-attachment; parts TOML `[layout]` schema.
- `layout-solving`: The `Layout` IR in core; the `copperleaf-layout` crate and its `solve()` pipeline (translate → place → route → DRC → report); the Topola adapter boundary; determinism requirements; unrouted-net reporting; zone generation for `Plane` nets.

### Modified Capabilities
- `erc-and-synthesis`: The `Constraint` enum loses `NetClass` and `Creepage` (moved to `LayoutConstraint`); the documented variant list is updated to the electrical-only set.
- `component-metadata`: The `Component` trait gains `layout_constraints()`; `CompiledComponent` gains a `layout` field.
- `board-compile-pipeline`: `CompiledBoard` and `Net` gain `layout` fields; the `Backend` trait gains the provided `emit_with_layout()` method; lowering resolves `LayoutConstraint::NetClass` into `Net.class`.
- `kicad-backend`: The PCB emitter consumes `Layout` (placements, segments, vias, zones) when provided via `emit_with_layout()`, falling back to `auto_place` + ratsnest otherwise; net-class emission consumes the now-populated `Net.class`.
- `deterministic-ids`: Determinism guarantee extended to layout solving for a fixed seed.

## Impact

- **`crates/core`**: `LayoutConstraint` enum and `Layout` IR (`placements`/`tracks`/`vias`/`zones`); narrowed `Constraint`; `Component::layout_constraints()`; `Board` layout-directive builder methods; `Backend::emit_with_layout()` provided method; `layout` fields on `CompiledBoard`/`CompiledComponent`/`Net`; new `BoardSide`/`Region`/`LayerSet` vocabulary types.
- **`crates/compile`**: Net-class resolution into `Net.class`; `PlaceNear` auto-attachment during decoupling synthesis; `layout` fields populated during lowering.
- **`crates/layout`** (new): `solve()` pipeline, `topola_adapter`, internal DRC, `SolveOptions`/`LayoutReport`/`LayoutError`. New workspace dependency: `topola = "0.1"` (MIT OR Apache-2.0 — compatible with the project's MPL-2.0).
- **`crates/backend-kicad`**: `emit_with_layout()` implementation; segment/via/zone emission; placements from `Layout`; `auto_place` retained as fallback only; revived net-class emission.
- **`crates/part-codegen`, `crates/cli`, `parts/*`**: TOML `[layout]` section; codegen emits `layout_constraints()`; manifests regenerated (breaking schema change for `net_class`/`creepage` keys).
- **Downstream board projects**: `Constraint::NetClass`/`Constraint::Creepage` references migrate to `LayoutConstraint`; `main.rs` gains two lines (`solve()` + `emit_with_layout()`) to opt into routed output.
- **Golden tests**: Phase 1 regenerates codegen goldens (enum rename only); Phase 2 goldens byte-identical when no layout is passed; Phase 3 adds new layout goldens with structural assertions for copper.
- **Risk posture**: Topola is 0.1.0 and sparsely documented (~7% rustdoc coverage). The adapter seam (design D3) contains this; the `Layout` IR is owned by core, so no backend or board project ever names a Topola type.
