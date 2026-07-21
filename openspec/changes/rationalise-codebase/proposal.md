## Why

A codebase assessment found that Copperleaf's core concepts are modelled three to five times each: pad geometry lives in at least six structs (`Pin`'s physical half, `PinBuilder`, `MechanicalPad`, `fp_parser::PadDef`, `fp_geom::PadGeom`, `codegen::PinDef`/`MechanicalDef`/`ThermalViaDef`), component metadata in four (the `Component` trait's 12 getters, `CompiledComponent`'s 12 fields, `codegen::ComponentMeta`, and two copies of the trait→struct conversion), and connectivity in three representations joined by string name-matching. `backend-kicad` and `part-codegen` implement two parallel emit pipelines against two of these models, with subtly divergent defaulting rules. Net resolution in `compile` is spread across five functions with precedence rules split between them. Every new field or rule must be replicated across all copies, and several copies have already drifted (divergent pad defaults, a semantically overloaded `Pin.length`, a dead `nc` field).

Phase 0 (already merged) established the safety net: deterministic emission (no `HashMap` iteration in output paths) and golden-file tests characterising current behaviour for every parts crate. This change executes the remaining rationalisation — phases 1 to 6 — so that each concept has exactly one definition and the two pipelines become one.

## What Changes

- **Phase 1 — Mechanical dedup.** Single `deterministic_id` in core (backend re-exports); `string_value` moves onto `Sexpr`; one `fmt_f64`; delete the `role_to_pintype` alias; merge the two thermal-via containment helpers; collapse the four KiCad `(property …)` builders into one; collapse the three MECH-pin synthesis loops into one helper; one `CompiledComponent::from_component` constructor; shared CLI helpers for lib-id resolution, extension guards, and model embedding; one `required_fields(kind)` table consumed by validation, codegen, and serialisation; deduplicate `CompileError`. No behaviour change.
- **Phase 2 — One pad model.** **BREAKING**: introduce a single `Pad` struct in core carrying all pad geometry (number, pos, rotation, width, height, pad_type, shape, roundrect ratio, solder-mask margin, layers, drill). `Pin` keeps only electrical identity/specification plus `pad: Option<Pad>` and `symbol: Option<SymPin>` (schematic pin graphics). `MechanicalPad`, `fp_geom::PadGeom`, `ThermalViaDef`, and the physical half of `codegen::PinDef` are replaced by `Pad`. One `resolve_pad()` function owns all KLC defaulting rules, reconciling the current `fp_geom`/`fp_emitter` divergence. This eliminates the `Pin.length` overload (symbol stub length vs pad dimension).
- **Phase 3 — One component representation.** **BREAKING**: introduce `ComponentMeta` in core (symbol, footprint, datasheet, description, 3D-model data). The `Component` trait collapses from 12 getters to `pins()`, `meta()`, `constraints()`. `CompiledComponent` becomes `{ refdes, meta, pins, constraints }`. The TOML `Manifest` serde-maps onto the same core types, so `backend-kicad`'s dual emitters (`sym_emitter`/`fp_emitter` vs `lib_emitter`) merge into single functions over one input type — the two pipelines become one.
- **Phase 4 — Connectivity simplification.** **BREAKING**: `Connection.net` becomes an index into `CompiledBoard.nets` instead of a name string. Net resolution collapses from `resolve_net_overrides`/`merge_overrides`/`infer_voltage_from_pins`/`classify_net` into a single `resolve_net()` with explicit precedence (explicit override → power-pin `v_nom` consensus → ground fallback → `NO_VOLTAGE_SOURCE` error). One `Net::is_ground()` test everywhere. Compile builds a `BoardView` (pin→net index, connected set) once; ERC and emitters consume it instead of rebuilding lookups. `pwr_net()` self-connection is replaced by a first-class single-pin net API.
- **Phase 5 — CLI rationalisation.** `serialise()` delegates to derived serde TOML with a post-pass for ordering/TODO comments (single schema source of truth). `new`/`update` collapse into one merge pipeline over a `Source` enum. `KindEntry` flattens the shared electrical-fields struct. The seven identical `build.rs` files shrink to a shared function call. The `nc` field is honoured: codegen emits a no-connect attribute and ERC checks it (name-prefix matching retained as fallback).
- **Phase 6 — Model honesty & docs.** Refresh stale specs (e.g. `kicad-backend` describing `symbol()` S-expression embedding that no longer exists). Remove remaining stringly-typed lookups and sentinels. Add a `CompiledComponent` test builder to eliminate repeated 12-field test literals.
- Each phase lands as an independently mergeable unit with the golden net green; phases 2 and 3 additionally regenerate all parts crates and diff outputs.

## Capabilities

### New Capabilities
- `pad-model`: The unified `Pad`/`SymPin` model in core — field definitions, `resolve_pad()` defaulting rules (auto-row, drill, layers, shape, anchor normalisation), and conversions at the parser/codegen boundaries.
- `component-metadata`: `ComponentMeta` in core; the collapsed `Component` trait; `CompiledComponent`'s new shape; TOML `Manifest` mapping onto core types; the single symbol/footprint emission path.

### Modified Capabilities
- `code-only-components`: `Component` trait shape changes (`meta()` replaces per-field getters; `Pin` restructured around `Pad`/`SymPin`).
- `power-spec`: `PinBuilder` loses physical-field setters (moved to `Pad` construction); electrical builder methods unchanged.
- `typed-pin-refs`: `pwr_net()` self-connection helper replaced by a first-class single-pin net API.
- `inferred-nets`: net identity becomes indexed; resolution becomes a single pass with explicitly stated precedence; `net_overrides` leaves the edge-indexed parallel array.
- `board-compile-pipeline`: `CompiledBoard`/`CompiledComponent` shape changes; pipeline produces a `BoardView` for consumers; one `CompileError` type.
- `compile-report`: Reconciles the spec with the implementation — removes the never-implemented `SynthCap`/`caps_synthesised` requirements (synthesised capacitors are audited via the compiled board's component list and the `DECOUPLE:SUMMARY` diagnostic).
- `erc-and-synthesis`: ERC consumes `BoardView` instead of rescanning connections; NC detection uses the honoured `nc` pin attribute with name-prefix fallback.
- `kicad-backend`: one emission path for symbols/footprints regardless of source (manifest vs compiled board); output is byte-identical across processes.
- `footprint-parser`: `PadDef` gains bidirectional conversion to the core `Pad` type.
- `parts-cli`: TOML schema gains a `pad` representation and the honoured `nc` attribute; `serialise()` derives from the schema; `new`/`update` share one merge pipeline.
- `deterministic-ids`: emission output SHALL be byte-identical across processes (documents the Phase 0 fix and its regression test).

## Impact

- **`crates/core`**: Major model refactor — `Pad`, `SymPin`, `ComponentMeta`, restructured `Pin`/`PinBuilder`/`Component`/`CompiledComponent`, indexed `Connection.net`, `BoardView`, single `CompileError`, `Net::is_ground()`.
- **`crates/compile`**: Net resolution collapses into one pass; decoupling synthesis and capacitor construction use the shared component constructor; `BoardView` produced here.
- **`crates/backend-kicad`**: `fp_geom` shrinks to defaulting rules + S-expression helpers; `sym_emitter`/`fp_emitter`/`lib_emitter` merge into one emission path; parsers convert to core `Pad` at the boundary.
- **`crates/part-codegen`**: `PinDef`/`MechanicalDef`/`ThermalViaDef` replaced by core types with serde; template updated for `Pad`/`SymPin`; one `required_fields` table.
- **`crates/part-macro`**: No functional change (regenerated output changes shape via the template).
- **`crates/cli`**: `manifest.rs` loses field-by-field mapping and the hand-rolled serialiser; `new.rs`/`update.rs` share one pipeline; `kindmap.rs` flattens shared fields.
- **`parts/*`**: All TOML definitions re-emitted in the new schema; generated Rust regenerates against the new core API; `build.rs` files deduplicated.
- **Downstream board projects** (e.g. `halow-sta`): update part construction and any `Component` impls to the new trait; pin constants/`PinRef` ergonomics unchanged.
- **Golden tests**: Phases 1, 4, 5 must be byte-for-byte no-ops against the golden net; phases 2 and 3 are expected to change goldens only where the reconciled defaulting rules differ (each such diff reviewed explicitly).
