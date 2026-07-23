## 1. Phase 1 — Constraint split (breaking; golden: enum-rename diffs only)

- [ ] 1.1 Define `LayoutConstraint`, `BoardSide`, `Region`, and `LayerSet` in `crates/core/src/net.rs` (or a new `layout.rs` module); remove `NetClass` and `Creepage` from `Constraint`; update core re-exports
- [ ] 1.2 Add `layout: Vec<LayoutConstraint>` fields to `CompiledComponent`, `Net`, and `CompiledBoard`; add `Component::layout_constraints()` default-empty trait method; carry it through `CompiledComponent::from_component`
- [ ] 1.3 Add `Board` builder methods `place_at`, `place_near`, `keepout`, `assign_plane` with handle validation matching `connect()`
- [ ] 1.4 Migrate all `Constraint::NetClass`/`Constraint::Creepage` construction sites (parts crates, tests, codegen template) to `LayoutConstraint`
- [ ] 1.5 Update the parts TOML `Manifest` schema: move `net_class`/`creepage` into a `[layout]` table serde-mapping onto `LayoutConstraint`; reject the old keys with a naming error; update codegen to emit `layout_constraints()`
- [ ] 1.6 Attach `PlaceNear { target, max_radius: 5 mm }` to synthesised decoupling capacitors in `crates/compile` (named constant for the radius)
- [ ] 1.7 Regenerate all parts crates and codegen goldens; review diffs (expect enum renames only, no geometry/output changes)
- [ ] 1.8 Verify `cargo test --workspace`, clippy (`-D warnings`), fmt; confirm backend goldens are byte-identical

## 2. Phase 2 — Layout IR and backend consumption (golden: reviewed net-class diffs only)

- [ ] 2.1 Define the `Layout` IR in core (`Layout`, `Placement`, `Track`, `Via`, `Zone`) with `Clone + Debug + serde`; re-export from crate root
- [ ] 2.2 Resolve `LayoutConstraint::NetClass` into `Net.class` during lowering in `crates/compile` (delete the eight `NetClass::default()` hard-codings for constrained nets); add a resolution-conflict diagnostic for nets with competing directives
- [ ] 2.3 Add the provided `Backend::emit_with_layout()` method defaulting to `emit()`
- [ ] 2.4 Implement `KiCad::emit_with_layout()`: placements/rotations/side from `Layout` (F/B layer swap and mirrored text for back-side components); decide full-outline vs region zones per design open question
- [ ] 2.5 Emit `(segment …)`, `(via …)`, and `(zone …)` nodes from `Layout` with deterministic UUIDs (seeded from net/layer/ordinal); unrouted nets emit no copper
- [ ] 2.6 Add backend tests: layout-supplied golden board (placements + segments + vias + zone), unrouted-net case, and byte-identical no-layout regression
- [ ] 2.7 Verify `cargo test --workspace`, clippy, fmt; review the revived power-net-class golden diffs explicitly and bless

## 3. Phase 3 — copperleaf-layout crate with embedded Topola

- [ ] 3.1 **Spike (time-boxed):** add `topola = "0.1"` to a scratch binary; determine (a) RNG seed control, (b) headless autoplacer usability, (c) in-memory `topola::board::Board` construction vs Specctra fallback, (d) coordinate units/resolution. Record findings in the change's design.md as an addendum; pick D4 primary or fallback path before proceeding
- [ ] 3.2 Scaffold `crates/layout` (`copperleaf-layout`) in the workspace: `SolveOptions { seed, effort, .. }`, `LayoutReport`, `LayoutError`, module layout per design D3
- [ ] 3.3 Implement `translate.rs`: `CompiledBoard` + layout constraints → adapter input model (pad geometry via `resolve_pad`, board outline, stackup layers, resolved net classes, fixed placements)
- [ ] 3.4 Implement `topola_adapter.rs`: build the topola board (outline, pads with net associations, layer stack, per-net width/clearance rules, keepouts); the only module with `use topola::*`
- [ ] 3.5 Placement stage: drive the autoplacer with fixed `PlaceAt` components locked; apply `PlaceNear`/`SameSide`; emit warnings for unsatisfiable directives; convert results to `Placement`s
- [ ] 3.6 Routing stage: exclude `Plane` nets (generate board-outline `Zone`s instead); autoroute remaining nets in deterministic net order; convert tracks/vias to the `Layout` IR; populate `unrouted`
- [ ] 3.7 Implement `drc.rs`: clearance/width checks per resolved `NetClass`, creepage minima, self-intersection detection; `LAYOUT:`-prefixed diagnostics into the report
- [ ] 3.8 Wire determinism per design D5 outcome (seed wiring, documented determinism, or vendored patch); add a two-process determinism test for `solve()`
- [ ] 3.9 Add golden tests: placement byte-exact; routing structural invariants (every net routed or in `unrouted`; DRC-clean) per spec; bless via `COPPERLEAF_BLESS=1`
- [ ] 3.10 End-to-end: migrate one real board project's `main.rs` to `solve()` + `emit_with_layout()`; open the result in KiCad, refill zones, and record DRC state as the acceptance note in this change

## 4. Close-out

- [ ] 4.1 Refresh spec Purposes marked TBD that this change touches (`board-compile-pipeline`, `deterministic-ids` if affected)
- [ ] 4.2 Document the new pipeline (`solve()` + `emit_with_layout()`, zone refill via `kicad-cli`) in crate-level docs and the board-project template
- [ ] 4.3 Final verification: `cargo test --workspace`, clippy (`-D warnings`), fmt, two-process determinism run for both `solve()` and emission
- [ ] 4.4 Archive this change and sync the updated specs into `openspec/specs/`
