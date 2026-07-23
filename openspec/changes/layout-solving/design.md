## Context

The pipeline today is `Board → compile() → CompiledBoard → Backend::emit()`. The KiCad backend's PCB emitter places footprints by row-packing (`auto_place` in `crates/backend-kicad/src/pcb.rs`) and emits zero copper. Meanwhile the `Constraint` enum (`crates/core/src/net.rs`) mixes two audiences:

- **ERC / synthesis** consume `Decoupling` (synthesis) and will consume `MaxJunction`/`ResonanceIndex` (analysis).
- **A layout solver** would consume `NetClass`, `Creepage`, `Impedance`, `LengthMatch`, `ReturnPath` — but nothing does. `Net.class` is hard-coded to `NetClass::default()` in all eight lowering sites in `crates/compile/src/lib.rs`, making the power-net-class path in the KiCad emitter dead code, and the remaining physical variants are stored but never read.

The targeted boards (RP2354/MM8108/W5500 module-style designs: low-speed digital, power, connectors, 2–4 layers) are the forgiving end of the routing problem. Topola (`topola` 0.1.0 on crates.io, MIT OR Apache-2.0, repository on Codeberg) provides an embeddable pure-Rust topological autorouter plus an autoplacer (`Autorouter`, `AutoplacerSchedule`, `board::Board`, `layout` module, Specctra support via `topola_specctra`). It is young — published 2026-06, ~6.5 kLOC, ~7% rustdoc coverage — so the integration must assume API churn and behavioural surprises, and contain both behind one module.

Constraints:
- Determinism is a project invariant (`deterministic-ids` spec, golden-file net). Layout output must be reproducible; Topola uses `rand` internally, so seed control is a hard requirement to validate before relying on byte-stable output.
- Golden tests MUST stay byte-identical whenever no layout is supplied — embedding a solver must not perturb the existing emission path.
- International English throughout.
- Topola licence (MIT OR Apache-2.0) is compatible with the workspace's MPL-2.0.

## Goals / Non-Goals

**Goals:**
- One vocabulary per audience: `Constraint` for electrical/behavioural intent (ERC, synthesis), `LayoutConstraint` for physical directives (solver, layout DRC).
- A core-owned `Layout` IR that any backend can consume without naming a Topola type.
- Routed KiCad output: placements, tracks, vias, and zones in `.kicad_pcb`, produced by `cargo run` with no intermediary tools.
- Graceful degradation: unrouted nets are reported and emitted as airwires; emission never hard-fails because the solver struggled.
- Repair net-class resolution so declared widths/clearances reach both the solver and the KiCad net classes.

**Non-Goals:**
- Interactive routing, push-and-shove UI, or a graphical editor of any kind.
- Differential pairs, impedance-controlled track geometry from stackup math, and length-matched serpentine tuning (the constraints remain declared intent; physical enforcement is follow-on work).
- Specctra DSN/SES import/export in copperleaf (Topola has `topola_specctra` if ever needed; the adapter boundary keeps this open without building it).
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

### D2: `Layout` is a core-owned IR, independent of Topola

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

Plain data, `Clone + Debug + serde`, ordered deterministically (construction order preserved; no hash-iteration). No Topola types appear. This is the same lesson as the pad-model rationalisation: the format boundary converts at the edge, the workspace speaks one type.

### D3: One adapter module contains all Topola contact

**Decision:** `crates/layout` is structured as:

- `lib.rs` — `solve(board: &CompiledBoard, options: &SolveOptions) -> Result<LayoutReport, LayoutError>`, orchestrating the stages.
- `translate.rs` — `CompiledBoard` + layout constraints → adapter input model (copperleaf units, copperleaf types).
- `topola_adapter.rs` — the **only** module with `use topola::*`: builds `topola::board::Board` (pads, outline, layer stack, net rules), drives autoplacement and autorouting, converts results back. All 0.x API churn lands here and nowhere else.
- `drc.rs` — copperleaf-side design-rule check of the solved `Layout` (clearance/width per resolved `NetClass`, creepage minima). Independent verification of a young dependency's output; doubles as the report's diagnostic source.
- `error.rs`, `report.rs` — `LayoutError`, `LayoutReport { layout, unrouted: Vec<NetIdx>, diagnostics: Vec<Diagnostic> }`.

The swap seam is the `Layout` IR itself plus this one module; no trait abstraction is introduced speculatively.

### D4: Embed Topola in-memory; no Specctra round-trip

**Decision:** Construct `topola::board::Board` directly from translated geometry. Topola 0.1.0's public surface (`Autorouter::new(board)`, `board::Board`, `layout` module) supports in-memory construction, and its Specctra support lives in a separate crate we do not need to touch. Serialising DSN and parsing SES would add two format boundaries and a file round-trip for zero benefit at our scale.

**Fallback:** if 0.1.0's in-memory board construction proves unusable (the ~7% rustdoc coverage makes this a real possibility), the adapter switches to emitting DSN / consuming SES *within `topola_adapter.rs` only* — the `Layout` IR and all specs are unaffected. The Phase 3 spike (task 3.1) decides this before further work.

### D5: Determinism is seeded; byte-stability of copper is verified, not assumed

**Decision:** `SolveOptions { seed: u64, effort: Effort, .. }` makes the seed explicit. The spike (task 3.1) must determine how Topola's `rand` usage is controlled; acceptable outcomes in order of preference: (1) Topola accepts a seeded RNG — wire ours in; (2) deterministic given fixed input regardless of seed — document; (3) neither — contribute seed control upstream or pin a vendored patch, and until then routing goldens assert **structural invariants only** (every net either routed or in `unrouted`; DRC diagnostics empty for the golden board), while placement goldens remain byte-exact. The spec wording reflects this: determinism is required *for a fixed seed*, and the test strategy is explicit about which outputs are byte-pinned.

### D6: Plane nets become zones, not routes

**Decision:** A net carrying `LayoutConstraint::Plane { layer }` is excluded from routing. The KiCad emitter turns it into a `(zone …)` covering the board outline on that layer; KiCad fills the pour on open (or via `kicad-cli` refill in CI). Topological routers handle pours poorly and GND is the canonical plane net on the targeted boards, so this is both simpler and electrically better. The solver reports plane-net pads whose zone cannot reach them as diagnostics; unrouted plane connections remain visible as airwires until KiCad refills.

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

- **Topola maturity (0.1.0, ~7% rustdoc coverage) → API churn and behavioural surprises.** → Contain all contact in `topola_adapter.rs` (D3); verify output with the independent internal DRC; pin the dependency version; documented plan B is a coarse grid router behind the same `solve()` signature — the `Layout` IR and specs are unaffected either way.
- **Seed control may not exist in Topola 0.1.0.** → Spike it before any routing code is written (task 3.1); D5 enumerates the ordered fallbacks (wire seed → document determinism → upstream patch → invariant-only goldens).
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

- Does Topola 0.1.0 expose RNG seed control, and is its autoplacer usable headless? (Task 3.1 spike answers both before routing work begins.)
- What coordinate resolution/units does Topola's board model use internally, and what rounding does the adapter need for KiCad's nm precision?
- Should `LayoutConstraint::Plane` support partial-outline pours (region instead of full board) in this change, or is full-outline sufficient for the first boards? (Task 2.4 decides; spec written for full-outline with region as a compatible extension.)
