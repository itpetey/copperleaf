## Context

The KiCad backend crate (`crates/backends/kicad`) currently contains a single 16-line placeholder: `emit_netlist_text`, which prints `(net "name")` per net and nothing else. It is explicitly labelled "not stable … for demos and tests only." The CLI `export` command is its only consumer.

The IR has since grown rich, serializable data that the backend ignores:

- `Design { nets, components, connections, constraints, diagnostics, graph }`
- `ComponentRecord { refdes, pins, constraints }` with `Pin { name, role, limits, sig }`
- `Connection { refdes, pin, net }` — the source of truth for connectivity
- `Net { name, kind, class: NetClass { min_width, clearance }, constraints }`
- `Role` enum maps naturally to KiCad pin types

The architecture doc (§6) scopes the KiCad backend as "schematic/pcb with footprints, netclasses, constraints, keepouts." Milestones: P0 = KiCad netlist, P1 = KiCad PCB export. This change delivers both, plus a minimal schematic emitter, turning the IR into KiCad-consumable output.

KiCad 6+ file formats are S-expression based. A key constraint: the IR carries **no geometry/placement** (no coordinates, no footprint graphics, no symbol art) and `ComponentRecord` carries **no part value/footprint lib_id** (the generic `Block` is erased at insertion). Emitters must therefore synthesize placement and use stand-ins for values.

## Goals / Non-Goals

**Goals:**

- `emit_netlist` — full KiCad S-expression netlist: `comp` entries + per-net `node` pin connections.
- `emit_pcb` — `.kicad_pcb` with net classes derived from `NetClass`/constraints and footprint stubs (pads + net assignment).
- `emit_schematic` — minimal structurally-valid `.kicad_sch` with auto-placed box symbols and net labels.
- Deterministic output (snapshot-testable), stable across runs.
- Graceful handling of empty designs.
- CLI exposure via `export`, `export-sch`, `export-pcb`.

**Non-Goals:**

- Copper routing / trace geometry (no autorouter; footprints have pads but no traces).
- Real symbol graphics or footprint pad layouts matching real parts (box symbols / generic pad rows only).
- Round-trip *parsing* of KiCad files back into the IR (export only).
- Full DRC-in-PCB emission (constraints become net classes; we don't emit KiCad custom rules).
- Using a KiCad symbol/footprint library (no `.pretty`/`.kicad_sym` integration).

## Decisions

### D1: A small S-expression builder rather than raw `format!`

**Decision:** Add a minimal `sexpr` module: an enum (`List(Vec<Node>), Atom(&'static str/str)`, plus raw/preformatted) with a `to_string()` that pretty-prints nested lists with indentation and a deterministic UUID helper.

**Rationale:** KiCad files are deeply nested S-expressions. Hand-rolled `format!` strings risk mismatched parens and unreadable snapshots. A builder guarantees structural validity and produces consistently indented, diff-stable output. No external dependency needed (avoiding `kicad-sexpr`/`nom` crates per the "keep deps minimal" guideline).

**Alternative considered:** Use an existing S-expression crate. Rejected — none are KiCad-flavoured, and adding a parser dep for a pure emitter is overkill.

### D2: Netlist maps IR connections to `(net (node ...))` blocks

**Decision:** `emit_netlist` iterates `design.nets` (insertion order) to assign 1-based net codes, then groups `design.connections` by net name into `(node (ref) (pin) (pinfunction) (pintype))` entries. Pin type is derived from `Role` by looking up the pin on the component record. Components come from `design.components` as `(comp (ref) (value))`.

**Rationale:** `Connection` is the serializable source of truth for connectivity, so it drives the net section. Looking up the pin role gives KiCad useful `pintype` metadata (power_in, bidirectional, etc.) for ERC in KiCad.

**Value stand-in:** `ComponentRecord` has no value field (the `Block` is erased). The netlist `value` SHALL use the refdes's leading alphabetic prefix (e.g. `U1` → `U`, `C1` → `C`) as a minimal type stand-in. A future IR addition of a `value`/`footprint` field on `ComponentRecord` will replace this.

**Role → pintype mapping:** `PowerIn`/`Gnd` → `power_in`, `PowerOut` → `power_out`, `AnalogIn` → `input`, `AnalogOut` → `output`, `DigitalIO`/`DiffPos`/`DiffNeg` → `bidirectional`.

**Net code stability:** Codes are assigned in `design.nets` order; any net name appearing only in connections (not `add_net`'d) is appended, sorted alphabetically, to keep output deterministic.

### D3: PCB derives net classes from `NetClass` and emits footprint stubs

**Decision:** `emit_pcb` emits a `.kicad_pcb` with: version/generator/setup, a `(net <code> "<name>")` table, a always-present `Default` net class, additional `net_class` entries grouped by distinct `NetClass { min_width, clearance }` configs (width/clearance converted metres→mm via `as_base() * 1000.0`), a rectangular board outline, and one `footprint` per component with a pad per pin (pad net assigned from connections).

**Rationale:** `NetClass` maps directly onto KiCad's `net_class` width/clearance, which is the P1 deliverable ("KiCad PCB export"). Footprint stubs with pad-net assignment let the PCB carry connectivity even without routing.

**Net class grouping:** Nets sharing the same `(min_width, clearance)` config are grouped into one named class (e.g. `Power_0p3mm`). Nets with default `NetClass` join `Default`. This avoids one class per net.

**Footprint stubs:** Each component becomes a `footprint` with `lib_id "copperleaf:Generic"`, auto-placed on a grid (row-major, fixed pitch). Pads are `thru_hole circle` with default drill/size, laid out in a row; each pad's `(net <code> "<name>")` is set from the connections for that pin (or left unassigned if the pin is unconnected).

**Board outline:** A fixed rectangle (e.g. 100×80 mm) via `gr_line`/`gr_rect` so the file is self-contained.

### D4: Schematic emits a minimal, openable `.kicad_sch`

**Decision:** `emit_schematic` emits a structurally-valid KiCad 6 schematic: `version`/`generator`/`uuid`/`paper`/`title_block`, a single generic box `lib_symbol`, one `symbol` instance per component (auto-placed on a grid with `Reference`/`Value` properties), and a `(label "<netname>")` placed near each connected pin.

**Rationale:** Completes the "schematic" half of the backend's stated scope. Full wire routing and real symbol graphics are out of scope (Non-Goals); labels annotate nets so the schematic is at least navigable. This is the lowest-fidelity emitter and is clearly documented as such.

**Limitation acknowledged:** Without drawn wires, KiCad won't infer pin-to-pin connectivity from this schematic; the netlist is the high-fidelity connectivity artifact.

### D5: Deterministic UUIDs from a tiny hash

**Decision:** KiCad 6 instances require `(uuid "8-4-4-4-12")` strings. Emit deterministic UUIDs via a small FNV-1a hash of a stable seed string (e.g. `"sch:U1"`, `"pcb:U1"`), formatted into UUID layout. No `uuid` crate dependency.

**Rationale:** Snapshot tests require byte-stable output; random UUIDs would break golden files. FNV-1a is ~10 lines, dependency-free, and sufficient for unique-per-entity deterministic IDs. Collisions are astronomically unlikely at design scale.

### D6: Remove `emit_netlist_text`, update the single consumer

**Decision:** Delete `emit_netlist_text`; add `emit_netlist`/`emit_schematic`/`emit_pcb`. Update `cmd_export` to call `emit_netlist`. Add `export-sch`/`export-pcb` CLI dispatch.

**Rationale:** The only caller is the CLI `export` path (small workspace, confirmed by the single `use backend_kicad` import). Keeping a deprecated alias would perpetuate the "not stable" placeholder. Marked BREAKING in the proposal.

## Risks / Trade-offs

- **[KiCad parser strictness]** Hand-written S-expressions may be subtly invalid for a specific KiCad version (whitespace, required fields, UUID format). → Mitigate: follow the KiCad 6 file format spec closely; add snapshot tests asserting key structural tokens (`(kicad_pcb`, `(net_class`, `(uuid`); document that full KiCad-open validation is a future integration-test concern.
- **[No part values in IR]** Netlist `value` and PCB footprint lib_id are stand-ins (refdes prefix / `copperleaf:Generic`). → Mitigate: document clearly; this is an IR limitation, not a backend bug. A future `ComponentRecord.value`/`footprint` field upgrades all emitters at once.
- **[Schematic low fidelity]** The `.kicad_sch` opens but lacks real connectivity. → Mitigate: scope as "structural/minimal" in specs and README; the netlist remains the authoritative connectivity export.
- **[Deterministic UUID collisions]** FNV-1a at design scale is safe, but two entities hashing identically would produce duplicate UUIDs (KiCad rejects dupes). → Mitigate: seed strings include entity type + refdes/name, guaranteeing uniqueness by construction.
- **[Unit conversion]** PCB uses mm; `Qty<Meter>::as_base()` returns metres. → Mitigate: single `mm_of(qty)` helper (`qty.as_base() * 1000.0`) used everywhere; tested with known values (e.g. `0.3.mm()` → `0.3`).

## Open Questions

- Should `ComponentRecord` gain `value: String` and `footprint: Option<String>` fields as part of this change, so the netlist/PCB carry real values? **Leaning no** — that's an IR change better scoped separately; the stand-ins are acceptable for v1 and the emitters are structured to adopt the field trivially when it exists.
- Board outline dimensions: fixed 100×80 mm vs. derived from component count. **Leaning fixed** for v1 determinism; auto-sizing can come with geometry IR.
