## 1. Extend part-codegen schema

- [x] 1.1 Make `Manifest`, `PinDef`, `ComponentMeta` `pub` in `crates/part-codegen/src/lib.rs`
- [x] 1.2 Add `Serialize` derive to `Manifest`, `PinDef`, `ComponentMeta`, `PinRow`, `ConstantRow`, `TemplateData`
- [x] 1.3 Add fields to `PinDef`: `pos: Option<(f64,f64)>`, `rotation: Option<f64>`, `length: Option<f64>`, `nc: Option<bool>` (all `#[serde(default)]`)
- [x] 1.4 Add `Serialize` to `CodegenError` variants or a separate `Serialize` impl if needed for CLI error paths
- [x] 1.5 Extend `builder_expr` to append `.pos(x, y).rotation(r).length(l)` when physical fields are present
- [x] 1.6 Extract a shared `validate(manifest: &Manifest) -> Vec<Diagnostic>` function (checks: required fields per kind, duplicate pin names, unresolved power pins, pin-name-to-const sanity)
- [x] 1.7 Add unit tests: `PinDef` with physical fields serialises/deserialises round-trip; `builder_expr` emits `.pos().rotation().length()`; `validate()` flags unresolved power pins
- [x] 1.8 Verify `cargo build --workspace` and `cargo test --workspace` pass

## 2. Footprint parser in backend-kicad

- [x] 2.1 Create `crates/backend-kicad/src/fp_parser.rs` with `PadDef { number, pos, rotation, width, height, pad_type }` struct
- [x] 2.2 Implement `parse_footprint(path: impl AsRef<Path>) -> Result<Vec<PadDef>, ParseError>` — reads `.kicad_mod`, uses `sexpr::parse`, extracts `(pad ...)` nodes
- [x] 2.3 Implement `parse_footprint_lib(dir: impl AsRef<Path>) -> Result<Vec<(String, Vec<PadDef>)>, ParseError>` — iterates `*.kicad_mod` in a `.pretty` directory
- [x] 2.4 Add `pub mod fp_parser` to `crates/backend-kicad/src/lib.rs` and re-export `PadDef`, `parse_footprint`, `parse_footprint_lib`
- [x] 2.5 Add unit tests: parse a sample `.kicad_mod` with known pads, verify `PadDef` fields; empty footprint returns empty list
- [x] 2.6 Verify `cargo build --workspace` and `cargo test --workspace` pass

## 3. CLI crate skeleton

- [x] 3.1 Create `crates/cli/Cargo.toml` — package `copperleaf-cli`, `[[bin]] name = "copperleaf"`, edition/license version workspace, deps: `clap` (derive, std, help, usage, error-context, suggestions), `toml`, `thiserror`, `copperleaf`, `copperleaf-part-codegen`, `copperleaf-backend-kicad`
- [x] 3.2 Add `crates/cli` to root `Cargo.toml` `[workspace].members`
- [x] 3.3 Add `clap` to root `Cargo.toml` `[workspace.dependencies]` with `default-features = false`
- [x] 3.4 Create `crates/cli/src/main.rs` with clap `Cli` struct, `Command` enum (`New`, `Update`), argument structs, dispatch to `run()` returning `ExitCode`
- [x] 3.5 Create stub modules: `new.rs`, `update.rs`, `kindmap.rs`, `manifest.rs` — each with a placeholder `pub fn run(...) -> Result<(), CliError>`
- [x] 3.6 Define `CliError` type wrapping `Diagnostic` and `io::Error`
- [x] 3.7 Verify `cargo build --workspace` passes

## 4. Kind-map implementation

- [x] 4.1 Implement built-in `pin_type → kind` mapping in `kindmap.rs` (table per design D5)
- [x] 4.2 Implement `--kind-map <FILE>` loader: parse TOML with `[by_type]` and `[by_name]` tables, merge over built-ins
- [x] 4.3 Implement resolution function: given a pin name and pin_type, return (kind, kind-args) with `by_name` taking precedence over `by_type` taking precedence over built-in
- [x] 4.4 Add unit tests: `power_in` → `pwr`, `gnd` → `gnd`, `clock` → `clk`, unknown type → `--default-kind` + warning, `by_name` override precedence

## 5. TOML manifest helpers

- [x] 5.1 Implement `manifest::serialise(manifest: &Manifest) -> String` in `manifest.rs` — produces TOML in the `[[pin]]` schema format with `# TODO` comments for unresolved power pins
- [x] 5.2 Implement `manifest::deserialise(input: &str) -> Result<Manifest, CodegenError>` — wrapper around `toml::from_str`
- [x] 5.3 Implement `manifest::merge_symbol(existing: &mut Manifest, symbol: &[PinDef], kindmap: &KindMap, default_kind: &str)` — fills kinds/names, appends new pins, preserves manual overrides
- [x] 5.4 Implement `manifest::merge_footprint(existing: &mut Manifest, pads: &[PadDef])` — sets pos/rotation/length, warns on unmatched pads
- [x] 5.5 Add unit tests: serialise → deserialise round-trip; merge_symbol preserves manual voltages; merge_footprint sets pos without clobbering kinds; placeholder name replaced

## 6. new command — symbol source

- [x] 6.1 Implement `new --symbol <FILE> --lib-id <ID>` in `new.rs`: parse symbol lib, find symbol, flatten extends, map pins via kindmap, build `Manifest`, serialise to `--out` or stdout
- [x] 6.2 Normalise `--lib-id` into a valid TOML filename (lowercase, underscores) and valid Rust struct name (PascalCase)
- [x] 6.3 Handle `--title` and `--description` flags for component metadata
- [x] 6.4 Emit `# TODO` comments for power pins missing voltages
- [x] 6.5 Error handling: `CLI:SYMBOL_NOT_FOUND` when `--lib-id` doesn't match, `CLI:IO` on file errors
- [x] 6.6 Add integration test: parse a sample `.kicad_sym`, generate TOML, deserialise with `Manifest`, assert pin names/kinds/positions match

## 7. new command — footprint source

- [x] 7.1 Implement `new --footprint <FILE> --lib-id <ID>` in `new.rs`: parse footprint, build `Manifest` with `PAD_<n>` names, `--default-kind` kinds, physical fields from pads
- [x] 7.2 Warn `CLI:ANON_PAD_NAMES` (informational) when names are synthesised
- [x] 7.3 Add integration test: parse a sample `.kicad_mod`, generate TOML, deserialise with `Manifest`, assert pad numbers/positions match

## 8. new command — datasheet stub

- [x] 8.1 Implement `new --datasheet <FILE>`: print `CLI:DATASHEET_STUB` diagnostic (Severity::Error, hint about future LLM capability), exit 1, no file written

## 9. update command — symbol and footprint

- [x] 9.1 Implement `update <PART_TOML> --symbol <FILE> --lib-id <ID>` in `update.rs`: load existing TOML, parse symbol, `merge_symbol`, write back to `--out` or overwrite
- [x] 9.2 Implement `update <PART_TOML> --footprint <FILE> --lib-id <ID>`: load existing TOML, parse footprint, `merge_footprint`, write back
- [x] 9.3 Implement `update <PART_TOML> --datasheet <FILE>`: hard-fail with `CLI:DATASHEET_STUB`
- [x] 9.4 Print `CLI:NEW_PIN` warnings for pins in source but not in TOML
- [x] 9.5 Print `CLI:UNMATCHED_PAD` warnings for pads with no matching pin
- [x] 9.6 Add integration tests: `new --symbol` then `update --footprint` on the result — verify physical fields populated without clobbering logical fields

## 10. Vendor crate scaffolding

- [x] 10.1 Implement `--crate <VENDOR>` flag in `new` command: create `parts/<vendor>/Cargo.toml` (package `copperleaf-parts-<vendor>`, `[lib] path = "lib.rs"`, workspace deps)
- [x] 10.2 Create `parts/<vendor>/lib.rs` with module doc and `use copperleaf_part_macro::build_component;` import
- [x] 10.3 Append `"parts/<vendor>"` to root `Cargo.toml` `[workspace].members`
- [x] 10.4 Write the generated TOML to `parts/<vendor>/<lib_id>.toml` when `--crate` is set
- [x] 10.5 Add integration test: run `new --symbol ... --crate testvendor --out part.toml`, verify crate files and root `Cargo.toml` member exist

## 11. CLI polish and diagnostics

- [x] 11.1 Ensure all user-facing output uses `copperleaf::Diagnostic` with `CLI:` namespace codes
- [x] 11.2 Verify exit codes: 0 on success, 1 on any error (warnings don't cause non-zero exit)
- [x] 11.3 Add `--help` and `--version` output via clap built-ins
- [x] 11.4 Verify International English in all user-facing strings and code comments (e.g. "analyse", "colour", "synthesise", "metres")
- [x] 11.5 Run `cargo clippy --workspace --all-targets -- -D warnings` and fix any issues
- [x] 11.6 Run `cargo fmt --all -- --check` and fix any issues

## 12. Round-trip and end-to-end tests

- [x] 12.1 Round-trip test: `new --symbol sample.kicad_sym → .toml → part-codegen::generate_component_to_string → syn parse` — verify generated Rust contains pos/rotation/length calls
- [x] 12.2 Merge test: `new --symbol` then `update --footprint` on the result — verify physical fields populated without clobbering logical fields
- [x] 12.3 Round-trip with existing parts: run `new --symbol` on the `parts/raspberrypi/rp2354a` symbol data and compare output to the hand-authored `rp2354a.toml`
- [x] 12.4 Verify `cargo build --workspace` and `cargo test --workspace` pass after all changes
- [x] 12.5 Verify no changes to `crates/core`, existing `parts/*` behaviour, or backend emit behaviour (workspace tests pass; only touched `crates/core` to fix pre-existing test/clippy issues in the working tree)
