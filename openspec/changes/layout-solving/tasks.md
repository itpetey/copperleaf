## 1. Phase 1 — Constraint split (breaking; golden: enum-rename diffs only)

- [x] 1.1 Define `LayoutConstraint`, `BoardSide`, `Region`, and `LayerSet` in `crates/core/src/net.rs` (or a new `layout.rs` module); remove `NetClass` and `Creepage` from `Constraint`; update core re-exports
- [x] 1.2 Add `layout: Vec<LayoutConstraint>` fields to `CompiledComponent`, `Net`, and `CompiledBoard`; add `Component::layout_constraints()` default-empty trait method; carry it through `CompiledComponent::from_component`
- [x] 1.3 Add `Board` builder methods `place_at`, `place_near`, `keepout`, `assign_plane` with handle validation matching `connect()`
- [x] 1.4 Migrate all `Constraint::NetClass`/`Constraint::Creepage` construction sites (parts crates, tests, codegen template) to `LayoutConstraint`
- [x] 1.5 Update the parts TOML `Manifest` schema: move `net_class`/`creepage` into a `[layout]` table serde-mapping onto `LayoutConstraint`; reject the old keys with a naming error; update codegen to emit `layout_constraints()`
- [x] 1.6 Attach `PlaceNear { target, max_radius: 5 mm }` to synthesised decoupling capacitors in `crates/compile` (named constant for the radius)
- [x] 1.7 Regenerate all parts crates and codegen goldens; review diffs (expect enum renames only, no geometry/output changes)
- [x] 1.8 Verify `cargo test --workspace`, clippy (`-D warnings`), fmt; confirm backend goldens are byte-identical

## 2. Phase 2 — Layout IR and backend consumption (golden: reviewed net-class diffs only)

- [x] 2.1 Define the `Layout` IR in core (`Layout`, `Placement`, `Track`, `Via`, `Zone`) with `Clone + Debug + serde`; re-export from crate root
- [x] 2.2 Resolve `LayoutConstraint::NetClass` into `Net.class` during lowering in `crates/compile` (delete the eight `NetClass::default()` hard-codings for constrained nets); add a resolution-conflict diagnostic for nets with competing directives
- [x] 2.3 Add the provided `Backend::emit_with_layout()` method defaulting to `emit()`
- [x] 2.4 Implement `KiCad::emit_with_layout()`: placements/rotations/side from `Layout` (F/B layer swap and mirrored text for back-side components); decide full-outline vs region zones per design open question
- [x] 2.5 Emit `(segment …)`, `(via …)`, and `(zone …)` nodes from `Layout` with deterministic UUIDs (seeded from net/layer/ordinal); unrouted nets emit no copper
- [x] 2.6 Add backend tests: layout-supplied golden board (placements + segments + vias + zone), unrouted-net case, and byte-identical no-layout regression
- [x] 2.7 Verify `cargo test --workspace`, clippy, fmt; review the revived power-net-class golden diffs explicitly and bless

## 3. Phase 3 — placement-only `copperleaf-layout` crate (routing deferred)

- [x] 3.1 Scaffold `crates/layout` (`copperleaf-layout`) in the workspace: `SolveOptions { seed, effort, .. }`, `LayoutReport`, `LayoutError`, module layout per design D3
- [x] 3.2 Implement `translate.rs`: `CompiledBoard` + layout constraints → placement input model (pad geometry via `resolve_pad`, board outline, stackup layers, resolved net classes, fixed placements)
- [x] 3.3 Placement stage: honour `PlaceAt` components; produce deterministic placements and emit plane zones for `Plane` nets
- [ ] 3.4 Honour `PlaceNear` and `SameSide` in the placer; unsupported placement directives currently have no effect
- [ ] 3.5 Emit `LAYOUT:UNSUPPORTED_DIRECTIVE` diagnostics for directives the placement-only solver cannot honour (`PlaceNear`, `SameSide`, `Keepout`, and routing-only constraints)
- [x] 3.6 Implement `drc.rs`: clearance/width checks per resolved `NetClass`, creepage minima, self-intersection detection; `LAYOUT:`-prefixed diagnostics into the report
- [x] 3.7 Wire determinism: seed wiring and a two-process determinism test for `solve()`
- [x] 3.8 Add placement golden tests (byte-exact placements and generated plane zones)
- [ ] 3.9 Routing stage (deferred): no autorouter exists. Do not claim routed copper, unrouted-net reporting, or Topola integration until a separate change implements and tests it.
- [ ] 3.10 Routing golden tests (deferred with 3.9)
- [ ] 3.11 End-to-end placement acceptance: migrate one real board project to `solve()` + `emit_with_layout()` with placement-only layout, open the result in KiCad, and record DRC state as the acceptance note

## 4. Close-out

- [x] 4.1 Refresh spec Purposes marked TBD that this change touches (`board-compile-pipeline`, `deterministic-ids` if affected)
- [x] 4.2 Document the placement-only pipeline (`solve()` + `emit_with_layout()`, zone refill via `kicad-cli`) in crate-level docs and the board-project template
- [x] 4.3 Final verification: `cargo test --workspace`, clippy (`-D warnings`), fmt, two-process determinism run for placement and emission
- [ ] 4.4 Archive this change and sync the updated specs into `openspec/specs/` **only after 3.4, 3.5, 3.9, 3.10, and 3.11 are resolved or explicitly descoped**
