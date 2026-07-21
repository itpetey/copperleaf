## 1. Phase 1 — Mechanical dedup (golden: byte-identical)

- [x] 1.1 Remove `deterministic_uuid` from `backend-kicad/src/sexpr.rs`; re-export core's `deterministic_id` and update all call sites
- [x] 1.2 Move `string_value` onto `Sexpr` (e.g. `Sexpr::as_string()`); delete the copies in `sym_parser.rs` and `fp_parser.rs`
- [x] 1.3 Consolidate `fmt_f64` into one shared location; delete the copies in `part-codegen` and `cli/manifest.rs`
- [x] 1.4 Delete the `role_to_pintype` alias; use `role_to_pin_type` everywhere
- [x] 1.5 Merge `cli/update.rs::is_thermal_via` and `cli/manifest.rs::find_containing_pad` into one pad-containment helper
- [x] 1.6 Replace the four `(property …)` builders (`schematic.rs`, `sym_emitter.rs` ×2, `lib_emitter.rs`) with one parameterised helper in `common.rs`
- [x] 1.7 Extract the MECH-pin synthesis loop (currently in `schematic.rs::layout_for_comp`, `lib_emitter.rs::symbol_def_sexpr`, `netlist.rs::components_node`) into one shared helper
- [x] 1.8 Add `CompiledComponent::from_component(refdes, &dyn Component)`; use it in `compile_components` and `make_capacitor_component`
- [x] 1.9 CLI: extract shared `resolve_lib_id()`, `check_extension()`, and `embed_model()` helpers; remove the 4×/2×/3× copies in `new.rs`/`update.rs`
- [x] 1.10 Delete the dead `let _flattened = flatten_extends(...)` call in `new.rs`; reconcile `new`/`update` to the same extends-resolution path
- [x] 1.11 Add one `required_fields(kind) -> &[&str]` table; consume it from `codegen::validate`, `builder_expr` error paths, and `cli/manifest.rs::missing_power_fields`
- [x] 1.12 Delete `copperleaf_compile::CompileError`; re-export the core `CompileError` in its place
- [x] 1.13 Delete `Copperleaf` dead code in `project.rs` (unused `emit_project` library params) or wire them up — per design D10, delete unless a consumer exists
- [x] 1.14 Verify `cargo test --workspace`, clippy (`-D warnings`), and fmt all pass with zero golden diffs

## 2. Phase 2 — One pad model (golden: reviewed diffs only)

- [ ] 2.1 Add characterisation tests pinning current `fp_geom::pad_from_pin` vs `fp_emitter::pad_from_pin_def` defaulting behaviour (pad_type inference, shape/drill/layers defaults, anchor normalisation)
- [ ] 2.2 Define `Pad`, `PadType`, `PadShape`, and `SymPin` in `core` (with serde derives); define `Pin.nc: bool`
- [ ] 2.3 Restructure `Pin`/`PinBuilder`: remove bare physical fields and setters; add `pad: Option<Pad>`, `symbol: Option<SymPin>` with `.pad()`/`.symbol()` builder methods
- [ ] 2.4 Implement `resolve_pad()`/`resolve_mech_pad()` and anchor normalisation in core, implementing design D2's reconciled defaults
- [ ] 2.5 Replace `MechanicalPad` with `Pad` throughout core (`Component::mechanical`, `CompiledComponent.mechanical`)
- [ ] 2.6 Migrate `fp_geom` to consume core `Pad` via `resolve_pad()`; delete `PadGeom` and `pad_from_pin`/`pad_from_mechanical`
- [ ] 2.7 Add `PadDef::to_pad`/`from_pad` conversions in `fp_parser` (typed enums at the boundary); delete `fp_emitter`'s `pad_from_pin_def`/`pad_from_mechanical_def`
- [ ] 2.8 Update `part-codegen` schema and template to emit `.pad()`/`.symbol()` construction; regenerate render goldens
- [ ] 2.9 Migrate CLI merge code (`manifest_from_footprint`, `merge_footprint`, `merge_symbol`) to build `Pad`/`SymPin` directly; delete field-by-field mapping
- [ ] 2.10 Migrate hand-written parts (passives `Capacitor`/`Resistor`/`Crystal`) to the new `Pin` API
- [ ] 2.11 Regenerate all parts crates; review every golden diff against D2's stated rule; bless and document the review
- [ ] 2.12 Verify `cargo test --workspace`, clippy, fmt; confirm the `Pin.length` overload is gone (symbol length only in `SymPin`)

## 3. Phase 3 — One component representation (golden: byte-identical)

- [ ] 3.1 Define `ComponentMeta` in core (with `EMPTY` constant) and restructure `CompiledComponent` to `{ refdes, meta, pins, mechanical, constraints }`
- [ ] 3.2 Collapse the `Component` trait to `pins()`/`meta()`/`mechanical()`/`constraints()`; migrate all impls (passives, tests, generated template)
- [ ] 3.3 Replace the codegen `ComponentMeta` with the core type; update the TOML schema to serde-map onto core types
- [ ] 3.4 Migrate `compile_components`/`make_capacitor_component` to `CompiledComponent::from_component` with the meta shape
- [ ] 3.5 Merge `sym_emitter` + `lib_emitter::symbol_def_sexpr` into one symbol emitter over the unified representation
- [ ] 3.6 Merge `fp_emitter` + `lib_emitter::footprint_def` into one footprint emitter over the unified representation
- [ ] 3.7 Regenerate all parts crates; verify goldens are byte-identical (no D2-style reconciliation expected in this phase)
- [ ] 3.8 Verify `cargo test --workspace`, clippy, fmt

## 4. Phase 4 — Connectivity simplification (golden: byte-identical)

- [ ] 4.1 Introduce `NetIdx` and change `Connection.net` to an index into `CompiledBoard.nets`; add `CompiledBoard::net(idx)`/`find_net(name)` helpers
- [ ] 4.2 Move net overrides off the edge-indexed parallel `Vec<RawNetOverride>` into a per-net override map built during grouping
- [ ] 4.3 Implement one-pass `resolve_net()` encoding the documented precedence (override → v_nom consensus → ground fallback → NO_VOLTAGE_SOURCE); delete `resolve_net_overrides`/`merge_overrides`/`infer_voltage_from_pins`/`classify_net`
- [ ] 4.4 Add `Net::is_ground()`; replace the three ground-detection rules (compile ×2, decoupling fallback) with it
- [ ] 4.5 Add `Board::net(pin) -> Result<NetHandle, CompileError>` for single-pin nets; delete `helpers::pwr_net` and the self-connection edge
- [ ] 4.6 Build the board view (pin→net index, connected set) once during compilation; expose it from `CompiledBoard`
- [ ] 4.7 Migrate ERC rules to the board view; delete `connected_pins`, `component_index`, and the `usize::MAX` sentinel
- [ ] 4.8 Migrate KiCad emitters (netlist, pcb, schematic) to the board view; delete per-emitter `by_net`/`pin_to_net`/`net_conns` maps and `build_net_codes`' defensive extras
- [ ] 4.9 Verify `cargo test --workspace`, clippy, fmt with zero golden diffs

## 5. Phase 5 — CLI rationalisation (golden: byte-identical + TOML round-trip)

- [ ] 5.1 Replace the hand-rolled `manifest.rs::serialise` with derived `toml::to_string` plus a `# TODO: fill …` post-pass; add round-trip goldens for every parts TOML before switching
- [ ] 5.2 Unify `new`/`update` into one merge pipeline over a `Source` enum (new = merge into empty manifest), reusing Phase 1's shared helpers
- [ ] 5.3 Flatten `KindEntry` onto the shared electrical-fields struct via `#[serde(flatten)]`
- [ ] 5.4 Honour `nc`: codegen emits the marker on `Pin`; ERC checks `pin.nc()` with name-prefix fallback; merge/auto-wire tooling skips `nc` pins via the flag
- [ ] 5.5 Replace the seven identical `parts/*/build.rs` files with a call into a shared helper
- [ ] 5.6 Verify `cargo test --workspace`, clippy, fmt; confirm TOML round-trip goldens pass for all parts

## 6. Phase 6 — Model honesty, docs, and close-out

- [ ] 6.1 Refresh stale spec prose (e.g. `kicad-backend` Purpose, all `Purpose: TBD` sections) to match the rationalised reality
- [ ] 6.2 Add a `CompiledComponent` test builder; delete the repeated 12-field test literals across ~8 test modules
- [ ] 6.3 Sweep for remaining stringly-typed lookups and sentinel values; remove or document survivors
- [ ] 6.4 Migrate the known downstream project (`halow-sta`) to the final API; verify its build and emission
- [ ] 6.5 Final verification: `cargo test --workspace`, clippy (`-D warnings`), fmt, and a two-process determinism run
- [ ] 6.6 Archive this change and sync the updated specs into `openspec/specs/`
