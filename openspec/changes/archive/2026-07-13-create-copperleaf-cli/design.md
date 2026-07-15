## Context

The Copperleaf workspace has all the ingredients for automated part creation except the glue. `backend-kicad` ships `sym_parser` that extracts pin definitions from `.kicad_sym` files, and `part-codegen` turns TOML manifests into Rust via `build_component!`. But a developer creating a new part still types the TOML by hand — pin name, pin number, pin kind, position — for every pin. The `kicad-backend` spec explicitly anticipated a "future generator CLI" for exactly this purpose.

The existing TOML schema is private to `part-codegen` as `Manifest`/`PinDef`/`ComponentMeta` (deserialise-only, no `Serialize`). The CLI needs both directions. The schema also lacks `pos`/`rotation`/`length`/`nc` fields — physical data that `sym_parser` already extracts and that `PinBuilder` already accepts but that the TOML→Rust pipeline doesn't carry.

Part creation has three input sources, each contributing different data:

| Source | Logical (names, kinds, electrical limits) | Physical (pos, rotation, length) |
|--------|-------------------------------------------|-----------------------------------|
| Symbol (`.kicad_sym`) | Pin names, pin types → kinds, no voltages | pos, rotation, length from symbol graphics |
| Footprint (`.kicad_mod`) | Pad numbers only (no names, no kinds) | pos, rotation, pad dimensions |
| Datasheet (PDF) | Everything | Everything |

Symbols and footprints are complementary — they merge by pin number. A complete part is typically `new --symbol` then `update --footprint`. Datasheets are a future capability requiring non-deterministic parsing.

## Goals / Non-Goals

**Goals:**
- Provide a `copperleaf-cli` binary with `new` and `update` commands for creating and enriching part TOML definitions.
- Create parts from KiCad symbols (`.kicad_sym`) and footprints (`.kicad_mod` / `.pretty`).
- Merge source data into existing parts by pin number, preserving manually-authored fields.
- Round-trip physical data (`pos`/`rotation`/`length`) through the TOML schema so generated Rust bakes it in at construction time, per `code-only-components`.
- Scaffold vendor parts crates in the canonical `parts/<vendor>/` shape.
- Expose the TOML manifest schema from `part-codegen` so the CLI and codegen share one contract.
- Add a footprint parser to `backend-kicad` alongside `sym_parser`.

**Non-Goals:**
- Datasheet parsing via LLM — non-deterministic, future capability. The CLI hard-fails with `CLI:DATASHEET_STUB` for now.
- A `generate` command — `build_component!` is the sole TOML→Rust path.
- Board compilation or emission — the CLI never touches `Board::compile()` or `Backend::emit()`.
- GUI or interactive prompts — the CLI is fully declarative (flags only).
- Replacing hand-authored parts — the CLI produces a starting point; manual refinement is expected.

## Decisions

### D1: Expose schema from `part-codegen`, no separate schema crate

**Decision:** Make `Manifest`, `PinDef`, `ComponentMeta` `pub` in `part-codegen` and add `Serialize` derives. The CLI depends on `copperleaf-part-codegen` for both reading and writing TOML. Add `pos: Option<(f64,f64)>`, `rotation: Option<f64>`, `length: Option<f64>`, `nc: Option<bool>` to `PinDef`. Extract a shared `validate(manifest) -> Vec<Diagnostic>` function.

**Rationale:** The TOML schema already lives in `part-codegen` as private serde structs. Extracting a separate crate would create a one-trick dependency; the consumer is always `part-codegen` (reader) or the CLI (writer), both of which already depend on or are part of the generation pipeline. Keeping it in `part-codegen` is the lowest-friction factoring.

**Alternative considered:** A `copperleaf-part-schema` crate with only serde structs — rejected as over-factoring. The schema has one writer (CLI) and one reader (codegen); a crate split adds indirection with no consumer benefit.

### D2: Two commands — `new` and `update` — with `--source` selectors

**Decision:**

```
copperleaf new --symbol <FILE> --lib-id <ID> [--out <FILE>] [--crate <VENDOR>] [options]
copperleaf new --footprint <FILE> --lib-id <ID> [--out <FILE>] [options]
copperleaf new --datasheet <FILE> [--out <FILE>]

copperleaf update <PART_TOML> --symbol <FILE> --lib-id <ID> [options]
copperleaf update <PART_TOML> --footprint <FILE> --lib-id <ID> [options]
copperleaf update <PART_TOML> --datasheet <FILE>
```

`new` creates a part TOML from one source. `update` merges new source data into an existing TOML by pin number. `--datasheet` hard-fails in both commands.

**Rationale:** Two commands, not three or four. `new` is "create from scratch"; `update` is "enrich what exists." The source is a flag, not a subcommand, because the workflow is the same regardless of source — the difference is which fields get populated. Keeping it to two commands avoids a combinatorial `init`/`import`/`generate`/`validate` surface.

**Alternative considered:** Separate `import`/`export`/`init` commands — rejected as over-decomposing a simple create-enrich workflow.

### D3: `update` merges by pin number, preserving manual overrides

**Decision:** `update` loads the existing TOML into a `Manifest`, then merges source data keyed on `PinDef.number` (the physical pin number, present in both symbols and footprints). Merge rules:

- `--symbol`: Fill `kind` and kind-args for pins where `kind` is absent or matches the `--default-kind` placeholder. **Do not overwrite** manually-set voltages, bandwidths, `nc`, or `notes`. Append pins present in the symbol but not the TOML (warn `CLI:NEW_PIN`). Update `name` only if the existing name is a placeholder (`PAD_<n>`).
- `--footprint`: Set/overwrite `pos`, `rotation`, `length` only. Leave all logical fields (`name`, `kind`, voltages, bandwidths, `notes`) untouched. Warn `CLI:UNMATCHED_PAD` for pads with no matching pin in the TOML.

**Rationale:** Pin number is the stable key shared between symbols and footprints. Names can differ between sources (symbol "1V2O" vs footprint "1"); pin numbers don't. Preserving manual overrides respects the iterative workflow where a developer fills in voltages after importing the symbol, then runs `update --footprint` without losing that work.

**Alternative considered:** Merge by pin name — rejected. Pin names differ between symbol and footprint; pin numbers don't.

### D4: Footprint parser as a `backend-kicad` module, not a new crate

**Decision:** Add `pub mod fp_parser` to `backend-kicad`, parallel to `sym_parser`. It uses the existing `sexpr::parse` to turn `.kicad_mod` / `.kicad_pcb` footprint files into `PadDef { number, pos, rotation, width, height, pad_type }`. Re-export `PadDef` and `parse_footprint` from `lib.rs`.

**Rationale:** `sexpr` already lives in `backend-kicad`. The footprint parser is the physical-world analogue of `sym_parser`, has the same dependencies, and follows the same "available for CLI, not used in compile/emit" constraint. A separate crate would duplicate the `sexpr` dependency and the re-export pattern.

**Alternative considered:** A `copperleaf-fp-parser` crate — rejected for the same reason as D1. One consumer (the CLI), one existing dependency home.

### D5: Kind-map as a heuristic with override loading

**Decision:** A built-in table maps KiCad `pin_type` strings to Copperleaf `kind` values:

| KiCad `pin_type` | Copperleaf `kind` | Notes |
|------------------|--------------------|-------|
| `power_in`, `power` | `pwr` | Voltages not in symbol — `# TODO` placeholder |
| `power_out` | `pwr_fixed` | Same — `# TODO` placeholder |
| `gnd`, `ground` | `gnd` | |
| `passive`, `unspecified` | `dio` | |
| `input`, `output`, `bidirectional`, `3state` | `dio` | |
| `clock` | `clk` | `bw_mhz` defaulted to 25 unless overridden |
| `no_connect` | `dio` + `nc = true` | |
| anything else | `--default-kind` (default `dio`) | Warns `CLI:UNKNOWN_PIN_TYPE` |

`--kind-map <FILE>` loads a TOML file with two tables: `[by_type]` (keyed on pin_type string) and `[by_name]` (keyed on pin name, higher precedence). User overrides merge over built-ins.

**Rationale:** The mapping is heuristic — KiCad's `pin_type` carries less information than Copperleaf's `kind` (no voltages, no bandwidth). The defaults are conservative; power pins always need manual fill-in. Allowing per-name overrides handles cases like a `power_in` pin that's actually a fixed 1.2V regulator output: `[by_name] 1V2O = { kind = "pwr_fixed", v = 1.2, i = 0.01 }`.

**Alternative considered:** No built-in map, require `--kind-map` always — rejected as hostile to first-run experience.

### D6: Datasheet hard-fails, not stubbed as success

**Decision:** `new --datasheet` and `update --datasheet` print `CLI:DATASHEET_STUB` (severity Error, hint "LLM-assisted datasheet parsing is a future capability") and exit 1. No skeleton TOML is written.

**Rationale:** Datasheet parsing is non-deterministic (varied PDF layouts, tables, graphs) and will involve an LLM. A silent skeleton would be misleading — the developer might think the CLI did something useful. A hard fail is honest and surfaces the capability boundary.

**Alternative considered:** Write a hand-fill skeleton with empty `[[pin]]` stubs — rejected as above.

### D7: CLI crate layout and error handling

**Decision:**

```
crates/cli/
├── Cargo.toml
└── src/
    ├── main.rs        # clap Cli, dispatch, diagnostics, exit codes
    ├── new.rs         # new command (symbol/footprint/datasheet dispatch)
    ├── update.rs      # update command (merge logic)
    ├── kindmap.rs     # built-in pin_type → kind heuristics + override loader
    └── manifest.rs    # TOML read/write/merge helpers around part-codegen::Manifest
```

All user-facing output uses `copperleaf::Diagnostic { code, severity, message, entities, hint }` with `Severity` and `NAMESPACE:RULE` codes, matching the compile-time ERC output. The CLI introduces the `CLI:` namespace (`CLI:SYMBOL_NOT_FOUND`, `CLI:UNKNOWN_PIN_TYPE`, `CLI:DATASHEET_STUB`, `CLI:NEW_PIN`, `CLI:UNMATCHED_PAD`, `CLI:IO`). Exit 0 on success, 1 on any error.

`clap` with `derive` feature, `default-features = false`, enabled features: `derive`, `std`, `help`, `usage`, `error-context`, `suggestions`.

**Rationale:** Reusing `Diagnostic` keeps output consistent across CLI and compile-time. A flat module layout (no `cmd/` indirection) matches the workspace's style for small crates. The `manifest.rs` helper isolates TOML serialisation so `new`/`update` focus on merge logic.

### D8: Physical fields in TOML schema and `builder_expr`

**Decision:** Extend `part-codegen::PinDef` with `pos: Option<(f64,f64)>`, `rotation: Option<f64>`, `length: Option<f64>`. Extend `builder_expr` to append `.pos(x, y).rotation(r).length(l)` when fields are present:

```rust
// for a pin with pos/rotation/length:
Pin::build("TXN").role(Role::AnalogIn).rf_limits()
    .pos(101.6, 12.7).rotation(90.0).length(2.54)
    .pin()
```

No change to `component.mustache` — the template emits `builders` strings verbatim, so `builder_expr` just produces longer strings.

**Rationale:** Physical data must be baked at construction time per `code-only-components`. The template doesn't need to know about physical fields; `builder_expr` already produces the full expression string. This keeps the round-trip (symbol → TOML → Rust) single-source.

## Risks / Trade-offs

- **[Kind-map heuristics are approximate]** KiCad pin types carry less information than Copperleaf kinds. Power pins always need manual voltage fill-in. → Mitigated by `# TODO` comments in output TOML and `--kind-map` overrides on a per-name basis. The CLI produces a starting point, not a finished part.

- **[Pin-number keying assumes stable pin numbering]** If a symbol and footprint disagree on pin numbering, `update --footprint` will mismerge. → Mitigated by `CLI:UNMATCHED_PAD` warning for pads with no match. The developer reviews CLI output before committing.

- **[Schema additions are backwards-compatible]** New optional TOML fields (`pos`/`rotation`/`length`/`nc`) don't break existing `.toml` files. → No risk; additive.

- **[CLI is the workspace's first binary]** No existing convention for binary crates, CLI dependencies, or exit codes. → This change establishes the convention: `clap` with `default-features = false`, `Diagnostic` for output, exit 1 on error.

- **[`part-codegen` gains `Serialize` derives]** `PinDef`/`Manifest`/`ComponentMeta` become `Serialize` + `Deserialize`. → Negligible cost; they already derive `Deserialize`. `Serialize` adds compile time for the derive but no runtime cost.

- **[Footprint parser duplicates `sym_parser` parsing patterns]** Both walk `Sexpr` trees extracting structured data. → Acceptable duplication; the node shapes differ enough that a shared extractor would add abstraction without reducing code.

- **[British English discipline]** The CLI introduces user-facing strings across many files. → Follow AGENTS.md: "analyse", "colour", "synthesise", "metres", "minimise". Code review and `AGENTS.md` enforcement covers this.

## Migration Plan

This is additive — no existing code changes behaviour. Implementation order:
1. Extend `part-codegen` schema (`pub` types, `Serialize`, new fields, `builder_expr`, `validate`).
2. Add `fp_parser` to `backend-kicad`.
3. Create `copperleaf-cli` crate skeleton + `Cargo.toml` wiring.
4. Implement `kindmap`, `manifest` helpers, `new` (symbol first), `update` (symbol first).
5. Add footprint support to `new`/`update`.
6. Add `--crate` scaffolding.
7. Add datasheet hard-fail.
8. Tests, clippy, fmt, round-trip validation.

No downstream projects need updating — existing `.toml` files remain valid.

## Open Questions

- **Footprint file formats:** KiCad 7+ uses `.kicad_mod` (single footprint) and `.pretty` directories (libraries of footprints). Should the CLI accept both, or only `.kicad_mod`? `.pretty` directory parsing is a small extension (iterate `*.kicad_mod` in the dir) but adds path-handling complexity. Proposed: accept `.kicad_mod` and `.pretty` (directory) in the first pass.
- **Kind-map file format:** TOML is consistent with the part definitions, but pin names as keys are awkward in TOML (quoting, special characters). Proposed: `[by_type]` and `[by_name]` tables with string keys, accepting the quoting overhead.
- **Vendor crate scaffolding and `Cargo.toml` member injection:** Editing the root `Cargo.toml` programmatically is straightforward (append a line) but fragile if the file is reordered. Proposed: append after the last `parts/` member, preserving formatting. If this proves brittle, a post-hoc `cargo fmt`-equivalent for TOML could be added.