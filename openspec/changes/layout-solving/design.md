## Context

The pipeline today is `Board → compile() → CompiledBoard → Backend::emit()`. The KiCad backend's PCB emitter places footprints by row-packing (`auto_place` in `crates/backend-kicad/src/pcb.rs`) and emits zero copper. Meanwhile the `Constraint` enum (`crates/core/src/net.rs`) mixes two audiences:

- **ERC / synthesis** consume `Decoupling` (synthesis) and will consume `MaxJunction`/`ResonanceIndex` (analysis).
- **A layout solver** would consume `NetClass`, `Creepage`, `Impedance`, `LengthMatch`, `ReturnPath` — but nothing does. `Net.class` is hard-coded to `NetClass::default()` in all eight lowering sites in `crates/compile/src/lib.rs`, making the power-net-class path in the KiCad emitter dead code, and the remaining physical variants are stored but never read.

The targeted boards (RP2354/MM8108/W5500 module-style designs: low-speed digital, power, connectors, 2–4 layers) are placement-friendly, but this change deliberately stops at placement. No autorouter is embedded or depended on; routing remains manual in KiCad.

Constraints:
- Determinism is a project invariant (`deterministic-ids` spec, golden-file net). Placement output must be reproducible and byte-exact.
- Golden tests MUST stay byte-identical whenever no layout is supplied — embedding a solver must not perturb the existing emission path.
- International English throughout.

## Goals / Non-Goals

**Goals:**
- One vocabulary per audience: `Constraint` for electrical/behavioural intent (ERC, synthesis), `LayoutConstraint` for physical directives (solver, layout DRC).
- A core-owned `Layout` IR that any backend can consume without naming an external solver type.
- Placement-aware KiCad output: placements and plane zones in `.kicad_pcb`, produced by `cargo run`.
- Explicit diagnostics for directives the placement-only solver does not honour.
- Repair net-class resolution so declared widths/clearances reach both the solver and the KiCad net classes.

**Non-Goals:**
- Interactive routing, push-and-shove UI, or a graphical editor of any kind.
- Any autorouter, track/via generation, differential pairs, impedance-controlled geometry, length-matched serpentine tuning, or Specctra import/export. These are deferred to a separate change.
- Replacing or removing `auto_place` — it stays as the no-layout fallback.
- Thermal/PDN analysis consuming `MaxJunction`/`ResonanceIndex` (unchanged by this change).

## Decisions

### D1: Split constraints at the electrical/physical seam

**Decision:** Narrow `Constraint` to electrical/behavioural intent and introduce `LayoutConstraint` for physical directives:

```rust
pub enum Constraint {          // electrical intent — ERC & synthesis
    Impedance { target: Qty<Ohm>, tol_pct: f64 },
    LengthMatch { group: String, skew_ps: f64 },
    ReturnPath { requires_plane: bool },
    Decoupling { values: Vec<Qty<Farad>>, per_pin: bool },
    ResonanceIndex { max: f64 },
    MaxJunction { temp: Qty<Celsius> },
}

pub enum LayoutConstraint {    // physical directives — solver & layout DRC
    NetClass { min_width: Qty<Meter>, clearance: Qty<Meter> },   // moved
    Creepage { min: Qty<Meter>, voltage: Qty<Volt> },            // moved
    PlaceAt { pos: (f64, f64), rotation: f64, side: BoardSide },
    PlaceNear { target: PlaceTarget, max_radius: Qty<Meter> },
    SameSide { group: String },
    Keepout { region: Region, layers: LayerSet },
    Plane { layer: usize },                 // net-level: dedicate layer as pour
}
```

`Impedance`/`LengthMatch`/`ReturnPath` stay in `Constraint`: they are electrical intent that ERC can reason about today; the solver may read them opportunistically in future. `NetClass`/`Creepage` move because their only meaningful consumers are physical (router rules, layout DRC).

**Attachment** mirrors the existing pattern — `constraints` exists on component, net, and board, so `layout: Vec<LayoutConstraint>` is added alongside all three (`CompiledComponent.layout`, `Net.layout`, `CompiledBoard.layout`), and the `Component` trait gains `fn layout_constraints(&self) -> Vec<LayoutConstraint>` (default empty). Board-level authoring uses typed builder methods (`board.place_at(handle, …)`, `board.keepout(…)`, `board.assign_plane(net, layer)`) rather than raw enum pushing, keeping `main.rs` readable.

**Alternatives considered:** (a) one enum with the solver ignoring irrelevant variants — rejected: that is today's ambiguity, and it lets board authors state directives that are silently inert; (b) three enums (electrical / placement / routing) — rejected as over-granular; placement vs routing is one solver conversation.

### D2: `Layout` is a core-owned IR

**Decision:**

```rust
pub struct Layout {
    pub placements: Vec<Placement>,   // one per component, by component index
    pub tracks: Vec<Track>,
    pub vias: Vec<Via>,
    pub zones: Vec<Zone>,
}
pub struct Placement { pub component: usize, pub at: (f64, f64), pub rotation: f64, pub side: BoardSide }
pub struct Track     { pub net: NetIdx, pub layer: usize, pub width: Qty<Meter>, pub path: Vec<(f64, f64)> }
pub struct Via       { pub net: NetIdx, pub at: (f64, f64), pub drill: Qty<Meter>, pub diameter: Qty<Meter>, pub layers: (usize, usize) }
pub struct Zone      { pub net: NetIdx, pub layer: usize, pub outline: Vec<(f64, f64)> }
```

Plain data, `Clone + Debug + serde`, ordered deterministically (construction order preserved; no hash-iteration). No external solver types appear. This is the same lesson as the pad-model rationalisation: the format boundary converts at the edge, the workspace speaks one type.

### D3: One placement module contains all solver contact

**Decision:** `crates/layout` is structured as:

- `lib.rs` — `solve(board: &CompiledBoard, options: &SolveOptions) -> Result<LayoutReport, LayoutError>`, orchestrating the stages.
- `translate.rs` — `CompiledBoard` + layout constraints → the input model (copperleaf units, copperleaf types).
- `placer.rs` — deterministic placement engine and plane-zone generation. It has no external solver dependency.
- `drc.rs` — copperleaf-side design-rule check of the solved `Layout` (clearance/width per resolved `NetClass`, creepage minima).
- `error.rs`, `report.rs` — `LayoutError`, `LayoutReport { layout, diagnostics: Vec<Diagnostic> }`.

The report has no routed-net or unrouted-net field because routing is not implemented. A future routing change can add one behind a spec update.

### D4: Routing is deferred, not simulated

**Decision:** The crate emits placements and plane zones only. Directives that would require routing or that the placer does not yet honour (`PlaceNear`, `SameSide`, `Keepout`, electrical routing constraints) produce `LAYOUT:UNSUPPORTED_DIRECTIVE` diagnostics. A future change can add a router behind the same `Layout` IR without changing the specs' placement contract.

### D5: Determinism is seeded and byte-exact for placements

**Decision:** `SolveOptions { seed: u64, effort: Effort, .. }` makes the seed explicit. The placer is deterministic for fixed input and seed; placement goldens are byte-exact and verified in two processes. There are no routing goldens because no routing is produced.

### D6: Plane nets become zones, not routes

**Decision:** A net carrying `LayoutConstraint::Plane { layer }` produces a `(zone …)` covering the board outline on that layer; KiCad fills the pour on open (or via `kicad-cli` refill in CI). No routing is attempted for any net. Plane-zone geometry is validated by layout DRC.

### D7: `Backend` gains a provided `emit_with_layout`; existing flow unchanged

**Decision:**

```rust
pub trait Backend {
    type Error;
    fn emit(&self, output_dir: impl AsRef<Path>, board: &CompiledBoard) -> Result<(), Self::Error>;
    fn emit_with_layout(&self, output_dir: impl AsRef<Path>, board: &CompiledBoard, layout: &Layout)
        -> Result<(), Self::Error>
    { self.emit(output_dir, board) }   // default: ignore layout
}
```

Non-breaking for other/future backends, keeps `main.rs` the sole source of truth, and makes opting in a two-line change. The KiCad backend implements it: placements and rotations come from `Layout` (including `side` → F/B layer swap and mirrored text), tracks/vias/zones become `(segment …)`, `(via …)`, `(zone …)` with deterministic UUIDs seeded from net/layer/ordinal. Without a layout, `emit()` behaves exactly as today — auto-place, ratsnest only — and every existing golden stays byte-identical.

### D8: Synthesis auto-attaches `PlaceNear` for decoupling caps

**Decision:** When decoupling synthesis creates a capacitor for a power pin, it attaches `LayoutConstraint::PlaceNear { target: that pin's component, max_radius: 5 mm }` to the synthesised component. This is the first cross-pipeline dividend of the split: an electrical rule (`Decoupling`) produces a physical directive the placer honours, with no board-author involvement. The radius is a named constant overridable via `SolveOptions` for now; per-pin radius control can join the `Decoupling` variant later if needed.

## Risks / Trade-offs

- **Placement quality and unsupported directives.** → Keep placement-only claims explicit; emit diagnostics for directives the solver does not honour; a future routing change can build on the same `Layout` IR.
- **In-memory board construction may be unusable at 0.1.0.** → D4's fallback keeps DSN/SES conversion inside the adapter module; no spec or IR impact.
- **Scope creep into a "general autorouter".** → Non-Goals exclude interactivity, diff pairs, and serpentine tuning; success is measured on the project's own board portfolio, not arbitrary boards.
- **`BoardSide`/`Region`/`LayerSet` are new core vocabulary.** → Kept minimal (front/back; rect/circle regions; layer bit-set) and shared by `Keepout`, `PlaceAt`, and future constraints rather than per-variant ad-hoc types.
- **Trade-off: planes as zones shifts pour responsibility to KiCad.** → Simpler and electrically better, but the emitted `.kicad_pcb` needs a refill step (`kicad-cli`) before DRC-clean fabrication output; documented in the crate docs and the board-project template.

## Migration Plan

1. **Phase 1 (breaking)** lands alone: `Constraint` split + `layout` fields + TOML `[layout]` schema + regenerated parts. Downstream board projects rename `Constraint::NetClass`/`Creepage` → `LayoutConstraint::…`; no behavioural change in output (goldens shift only by the enum rename in codegen output).
2. **Phase 2** is additive: `Layout` IR + `emit_with_layout` + net-class resolution repair. Emitted `.kicad_pcb` net classes may now include declared power classes — an intended, reviewed golden diff (previously dead code).
3. **Phase 3** is opt-in: board projects add `solve()` + `emit_with_layout()` to `main.rs`. `emit()` without layout is byte-identical to today, so adoption is per-project with no forced migration.
4. **Rollback:** each phase is independently revertable; Phase 3's crate is additive-only, so reverting it never touches emission paths used without a layout.

## Open Questions

- Should `LayoutConstraint::Plane` support partial-outline pours (region instead of full board) in this change, or is full-outline sufficient for the first boards? (Task 2.4 decides; spec written for full-outline with region as a compatible extension.)

## Routing addendum

Routing is not implemented in this change. There is no autorouter dependency,
no unrouted-net reporting, and no routed-copper golden. Tasks 3.9–3.10 record
the deferred work; archive remains blocked until they are resolved or descoped.
