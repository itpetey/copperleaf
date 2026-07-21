## Context

The Phase 0 groundwork is merged: emission is deterministic across processes (no `HashMap` iteration in output paths), and a golden-file net characterises current behaviour — board emission for every parts crate (28 files), `generate footprint`/`symbol` for every parts TOML (22 files), and codegen render output (11 files), all blessable via `COPPERLEAF_BLESS=1`.

The remaining work restructures the model so each concept has one definition. The root causes identified in the assessment:

1. `Pin` conflates three responsibilities — electrical specification, schematic symbol graphics, footprint pad geometry — and every downstream struct inherits the confusion (`MechanicalPad`, `PadDef`, `PadGeom`, `PinDef` each re-declare the pad half).
2. There is no canonical component representation: TOML manifest, `Component` trait, and `CompiledComponent` are three encodings with ad-hoc conversions, so emission logic exists twice (`sym_emitter`/`fp_emitter` vs `lib_emitter`) with divergent defaults.
3. Relationships are stringly-typed (nets and pins joined by name), so every consumer re-derives lookups and invariants are re-negotiated at each site (three ground-detection rules, defensive net-code patching, MECH numbering in three places).

Constraints:
- Golden tests MUST stay green byte-for-byte for pure refactors (phases 1, 4, 5); expected diffs in phases 2/3 are reviewed explicitly.
- Public API breaks are acceptable (pre-1.0, one known downstream), but `PinRef`/`PinHandle`/`Board` ergonomics for board authors stay unchanged.
- International English throughout.

## Goals / Non-Goals

**Goals:**
- One struct per concept: `Pad` for pad geometry, `ComponentMeta` for component metadata, one `CompileError`, one deterministic-ID helper, one emission path for symbols/footprints.
- `Pin` models electrical identity/specification only; physical data lives in `Pad` (footprint) and `SymPin` (symbol graphics).
- Indexed net identity; single-pass net resolution with precedence stated in one place; a `BoardView` consumed by ERC and emitters.
- TOML schema derives from core types via serde — one source of truth, no field-by-field mapping code.
- CLI `new`/`update` share one merge pipeline; the `nc` attribute becomes functional.

**Non-Goals:**
- New EDA features (no new ERC rules, constraint types, or emit formats beyond what exists).
- KiCad format changes — emitted files stay semantically identical (byte-identical where the change is a pure refactor).
- Layout/routing/DRC improvements (auto-place stays as-is).
- Proc-macro ergonomics (`#[derive(Component)]`) — separate future change.
- crates.io publishing hygiene.

## Decisions

### D1: Split `Pin` along its natural seam — electrical, footprint, symbol

**Decision:** Introduce in core:

```rust
pub struct Pad {           // footprint pad geometry (also used for mechanical pads)
    pub number: String,
    pub pos: (f64, f64),
    pub rotation: f64,
    pub width: f64,
    pub height: f64,
    pub pad_type: PadType,        // enum: Smd | ThruHole | NpThruHole | Connect
    pub shape: PadShape,          // enum: Rect | RoundRect(Option<f64>) | Circle | Oval
    pub solder_mask_margin: Option<f64>,
    pub layers: Option<String>,
    pub drill: Option<f64>,
}

pub struct SymPin {        // schematic symbol pin graphics
    pub pos: (f64, f64),
    pub rotation: f64,
    pub length: f64,
}

pub struct Pin {
    // electrical identity & specification (unchanged semantics)
    id: PinId, name: String, number: Option<String>,
    role: Role, power_spec: PowerSpec, decouple: bool, sig_spec: Option<SigSpec>,
    thermal_vias: Vec<ThermalVia>,
    nc: bool,                    // NEW: honoured no-connect marker (D9)
    pad: Option<Pad>,            // footprint geometry
    symbol: Option<SymPin>,      // schematic graphics
}
```

`MechanicalPad` is deleted; `CompiledComponent.mechanical` becomes `Vec<Pad>`. `PadType`/`PadShape` replace the stringly-typed fields (parsing happens at the KiCad boundary, not in the model).

**Rationale:** Today the same ~10 fields are declared in six structs and mapped field-by-field in five places. One `Pad` removes the declarations *and* the mappings. Enums for pad type/shape push validation to the parse boundary and make invalid states unrepresentable (e.g. roundrect ratio on a circle pad).

**Alternatives considered:** Keep strings for pad_type/shape — rejected: stringly-typed fields are one of the root causes (`"None"` marker sentinel, `eq_ignore_ascii_case` checks scattered across four call sites). Keep `MechanicalPad` as a separate struct — rejected: it differs from `Pad` only in being non-electrical, which the `pin_index: Option<usize>`/list membership already expresses.

### D2: One `resolve_pad()` owns all defaulting rules

**Decision:** A single function in core resolves a `Pin`'s pad to fully-populated geometry for emission:

```rust
pub fn resolve_pad(pin: &Pin, index: usize) -> Pad      // electrical pins
pub fn resolve_mech_pad(pad: &Pad) -> Pad               // mechanical pads (number normalisation, layer defaults)
```

It owns the KLC rules currently split between `fp_geom::pad_from_pin` and `fp_emitter::pad_from_pin_def`: auto-row position fallback (2.54 mm pitch), pad-type default, width/height fallback to symbol length, layer defaults by pad type, drill default, shape default (pad 1 rect, others circle for auto through-hole rows), and anchor normalisation (`normalise_anchor` moves alongside).

**Reconciliation of the known divergence:** `fp_geom` infers `pad_type` from `pos.is_some()` (SMD when a position is present), while `fp_emitter` defaults unconditionally to `"smd"`. The resolved rule: explicit `pad_type` wins; otherwise SMD iff the pin has an explicit position, else through-hole (the `fp_geom` behaviour — it is the superset of cases and matches every shipped parts TOML). This is the one place where Phase 2 may produce golden diffs; each diff is reviewed before blessing.

**Rationale:** Defaulting rules are policy; policy must live in exactly one place. Today the two pipelines can produce different footprints for the same part.

### D3: `ComponentMeta` and a collapsed `Component` trait

**Decision:**

```rust
pub struct ComponentMeta {
    pub symbol: Option<String>,
    pub footprint: Option<String>,
    pub datasheet: Option<String>,
    pub description: Option<String>,
    pub model_3d: Option<String>,
    pub model_3d_data: Option<String>,
    pub model_3d_rotation: (f64, f64, f64),
    pub model_3d_offset: (f64, f64, f64),
}

pub trait Component {
    fn pins(&self) -> &[Pin];
    fn meta(&self) -> &ComponentMeta { &ComponentMeta::EMPTY }
    fn mechanical(&self) -> &[Pad] { &[] }
    fn constraints(&self) -> Vec<Constraint> { vec![] }
}

pub struct CompiledComponent {
    pub refdes: String,
    pub meta: ComponentMeta,
    pub pins: Vec<Pin>,
    pub mechanical: Vec<Pad>,
    pub constraints: Vec<Constraint>,
}
```

The trait's 12 getters collapse to 4; `CompiledComponent` drops from 12 ad-hoc fields to 5. `CompiledComponent::from_component(refdes, &dyn Component)` is the single constructor, used by both `compile_components` and decoupling-capacitor synthesis.

**Rationale:** The getter explosion exists because each field was added independently. Every addition touches the trait, `CompiledComponent`, two construction sites, the codegen template, and ~8 test literals. Grouping by cohesion (metadata vs electrical vs geometry) stops the replication.

**Alternatives considered:** Keep per-field getters — rejected: it is the current pain. Associated `type Meta` — rejected as over-engineering; one concrete struct suffices.

### D4: Manifest serde-maps onto core types; one emission path

**Decision:** `part-codegen`'s `PinDef`/`MechanicalDef`/`ThermalViaDef` are replaced by core `Pin`/`Pad` shapes with serde derives (the TOML schema gains a `[pin.pad]` / `[pin.symbol]` representation). `fp_parser::PadDef` remains as parse IR but converts via `Pad::from_pad_def()` / `Pad::to_pad_def()` at the CLI boundary. With `Manifest` and `CompiledComponent` the same shape, the backend exposes:

```rust
pub fn emit_symbol(comp: &ComponentData) -> String       // was sym_emitter + lib_emitter::symbol_def_sexpr
pub fn emit_footprint(comp: &ComponentData, name: &str) -> String  // was fp_emitter + lib_emitter::footprint_def
```

used by both the CLI (`generate`) and board emission (`lib_symbols`, embedded footprints).

**Rationale:** The dual pipelines exist only because the two input types differ. Unify the types and the duplication becomes unrepresentable, not merely tidier.

### D5: Indexed net identity and one-pass resolution

**Decision:** `Connection.net` becomes `NetIdx(pub usize)` into `CompiledBoard.nets`. `NetId(String)` disappears from the compiled model (names remain display data on `Net`). Net resolution collapses into one function:

```rust
fn resolve_net(members: &[PinRef2], overrides: &[NetOverride], errors: &mut Vec<Diagnostic>) -> NetResolution
// precedence, in order:
//   1. explicit override (voltage/name from NetHandle)
//   2. power-pin v_nom consensus (conflict → NET:VOLTAGE_CONFLICT)
//   3. ground fallback (Gnd-role pins imply 0 V)
//   4. power net with no voltage → NET:NO_VOLTAGE_SOURCE error
```

`NetGrouping` shrinks to union-find + groups; `edges_for_net` is deleted (overrides move off the edge-indexed parallel array into a per-net map built during grouping). One `Net::is_ground()` (`v_nom ≈ 0`) replaces the three detection rules.

**Rationale:** Precedence spread across five functions is the "complex logic tree" of the assessment. Stated in one function, it becomes reviewable and unit-testable. Indexed identity deletes every name-join (`pin_to_net`, `by_net`, `net_conns`, ERC scans) and the defensive extras in `build_net_codes`.

### D6: `BoardView` computed once

**Decision:** Compile produces a `BoardView` alongside `CompiledBoard`:

```rust
pub struct BoardView<'a> {
    board: &'a CompiledBoard,
    net_of: HashMap<(usize, usize), NetIdx>,   // (component, pin index) → net
    connected: HashSet<(usize, usize)>,
}
```

ERC rules and KiCad emitters take `&BoardView` (or `CompiledBoard` exposes `fn view(&self)` building it on demand). `connected_pins`, `component_index`, per-emitter map building, and the `usize::MAX` sentinel are deleted.

**Rationale:** Every consumer currently rebuilds the same inverted index (netlist, pcb, schematic, three ERC rules), several of them O(n²) or worse.

### D7: First-class single-pin nets replace `pwr_net` self-connection

**Decision:** `Board::net(pin: PinHandle) -> NetHandle` registers a single-pin net without a fake edge; `helpers::pwr_net` is removed. Overrides key off `NetHandle` into a per-net override map (no parallel `Vec<RawNetOverride>` indexed by edge).

**Rationale:** The self-connection is a hack that forces union-find and edge accounting to absorb a meaningless edge.

### D8: `serialise()` derives from the schema

**Decision:** The hand-rolled 200-line TOML writer is replaced by `toml::to_string` on the (now serde-complete) manifest, followed by a small post-pass that emits `# TODO: fill …` comments for incomplete power pins. Field ordering is controlled by struct field order plus `#[serde(rename)]`/`default` attributes.

**Rationale:** Two serialisation implementations (derive + hand-written) must be kept in sync today; the hand-written one also duplicates `fmt_f64` and all 15 `ConstraintDef` fields.

### D9: `nc` becomes functional

**Decision:** `Pin` gains `nc: bool`; codegen emits it from the TOML `nc` field; `erc_nc_pin_connected` checks `pin.nc()` with the `NC`/`NC_*` name-prefix retained as a fallback for hand-written parts. `Board::connect` validation and `auto_wire`-style tooling skip `nc` pins via the flag.

**Rationale:** The field is currently dead data-flow (set by kindmap, serialised, never consumed). Honouring it is the honest mechanism; name matching stays as back-compat so existing parts are unaffected.

### D10: Sequencing and regression strategy

**Decision:** Phases land in order 1 → 6, each independently mergeable:

| Phase | Golden expectation |
|---|---|
| 1 (dedup) | byte-identical goldens |
| 2 (pad model) | diffs only where D2's reconciliation bites; each reviewed |
| 3 (component repr) | byte-identical goldens after D2 settles |
| 4 (connectivity) | byte-identical goldens |
| 5 (CLI) | byte-identical goldens; TOML round-trip goldens for serialise |
| 6 (docs/tests) | n/a |

After phases 2 and 3, all parts crates are regenerated and their TOML/Rust diffs reviewed. Every phase ends with `cargo test --workspace` green plus clippy/fmt.

**Rationale:** Small, individually revertable steps; the golden net turns "no regression" from an assertion into a diff.

## Risks / Trade-offs

- **[D2 reconciliation changes output for some part]** The `pad_type` default differs between the two pipelines today, so unifying must change one of them. → Mitigation: golden net characterises both pipelines per part before the change; adopt the `fp_geom` rule; review every blessed diff individually.
- **[`Pad` enums reject previously-accepted strings]** Vendor TOMLs may contain pad types/shapes outside the enum. → Mitigation: parse at the boundary with a clear error naming the offending value; `Custom(String)` variant only if a real part needs it (none shipped today).
- **[Trait collapse breaks all `Component` impls]** Hand-written impls (passives, tests, downstream) must move to `meta()`. → Mitigation: mechanical migration; generated impls update via the template; `ComponentMeta::EMPTY` keeps simple impls one line.
- **[Indexed `Connection.net` breaks the public compiled model]** Any consumer matching nets by name must switch to index + `nets[idx]`. → Mitigation: pre-1.0 crate, one known downstream; provide `CompiledBoard::net(idx)` and `find_net(name)` helpers to ease the transition.
- **[Serde-derived TOML output differs cosmetically]** Key ordering/line-wraps from `toml::to_string` differ from the hand-rolled writer. → Mitigation: TOML round-trip goldens in Phase 5; cosmetic diffs accepted once reviewed, semantic diffs are not.
- **[Bigger `Pin` construction surface]** Builders must construct `Pad`/`SymPin` sub-structs. → Mitigation: `PinBuilder` keeps convenience setters that populate the sub-structs internally, so part definitions barely change.
- **[Scope: six phases in one change]** Large changeset. → Mitigation: phases are sequenced as independent task groups; any phase can ship without the later ones (phase 1 alone is already a win).

## Migration Plan

1. **Phase 1** — pure dedup behind green goldens; no API change. Ship immediately.
2. **Phase 2** — land `Pad`/`SymPin` + `resolve_pad()`; migrate core, backend, codegen, CLI, parts TOMLs (re-emit); review golden diffs against D2's stated rule; bless.
3. **Phase 3** — land `ComponentMeta` + trait collapse; regenerate parts; goldens must be byte-identical at this point.
4. **Phase 4** — indexed nets + `BoardView`; goldens byte-identical.
5. **Phase 5** — CLI pipeline merge + serde serialise + `nc`; TOML round-trip goldens.
6. **Phase 6** — spec refresh (archive this change with updated specs), test builder, sentinel removal.

Rollback: each phase is a separate commit/PR; reverting a phase restores its predecessor's goldens. Downstream (`halow-sta`) pins to the pre-change revision until phases 2–4 land, then migrates once against the final API.

## Open Questions

- **Module layout for `Pad`/`SymPin`:** new `core::geom` module vs extending `core::pin`. Leaning to `core::pin` (cohesive, one import path) but `geom` is tidier once board-outline geometry arrives.
- **Fate of `emit_project`'s library parameters:** currently dead (`KiCad::emit` passes `&[]`/`None`). Wire them up properly or delete the parameters — decide in Phase 1 (delete unless a consumer exists).
- **`SymbolDef`/`PadDef` parse IR retention:** keep both as KiCad-format-faithful parse results, or parse directly into `Pad`/`SymPin`? Leaning to keeping parse IR + conversions (single responsibility), revisit in Phase 2.
- **Regulator output voltage model:** carried over from the core-redesign change; still deferred — `PowerOut` nominal output modelling is not required by this rationalisation.
