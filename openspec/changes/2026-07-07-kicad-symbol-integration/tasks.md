## 1. IR changes: Pin and ComponentRecord fields

- [ ] 1.1 Add `pos: Option<(f64, f64)>` and `rotation: Option<f64>` to `Pin` in `crates/ir/src/lib.rs`
- [ ] 1.2 Derive `Default` on `Pin` (or provide a sensible default impl)
- [ ] 1.3 Update `Pin::new()` to initialize `pos: None, rotation: None`
- [ ] 1.4 Add `kicad_symbol: Option<String>` to `ComponentRecord`
- [ ] 1.5 Add `fn kicad_symbol(&self) -> Option<&str> { None }` to the `Block` trait as a default method
- [ ] 1.6 Update `Design::add_component` to copy `kicad_symbol()` into the `ComponentRecord`
- [ ] 1.7 Update all existing `Pin { ... }` struct literals in `crates/parts/src/lib.rs` to include `pos: None, rotation: None` (or use `..Default::default()`)
- [ ] 1.8 Update test `Pin` literals in `crates/ir/src/lib.rs`, `crates/backends/kicad/src/schematic.rs`, `crates/analysis/src/lib.rs`, and `crates/edsl/src/lib.rs`
- [ ] 1.9 Add unit tests: component without `kicad_symbol` gets `None`, component with `kicad_symbol()` override gets `Some(...)`
- [ ] 1.10 Run `cargo build` and `cargo test` to verify IR compiles and all existing tests pass

## 2. S-expression parser in sexpr module

- [ ] 2.1 Add `pub fn parse(input: &str) -> Result<Sexpr, ParseError>` to `crates/backends/kicad/src/sexpr.rs`
- [ ] 2.2 Implement a tokenizer: atoms, quoted strings (with escape handling), `(`, `)`, and `#`-prefixed line comments
- [ ] 2.3 Implement recursive-descent list/atom parsing into `Sexpr::List`/`Sexpr::Atom`
- [ ] 2.4 Define `ParseError` type (unexpected EOF, unmatched paren, bad escape)
- [ ] 2.5 Add unit tests: parse simple lists, nested lists, quoted strings, comments, whitespace tolerance, error cases
- [ ] 2.6 Verify symmetry: `parse(&sexpr.to_string())` round-trips for representative KiCad fragments

## 3. Symbol library parser module

- [ ] 3.1 Create `crates/backends/kicad/src/sym_parser.rs` with `SymbolDef` and `PinDef` structs
- [ ] 3.2 Implement `fn parse_symbol_lib(input: &str) -> Result<Vec<SymbolDef>, ParseError>` using `sexpr::parse`
- [ ] 3.3 Walk the parsed S-expression tree: find `(symbol ...)` nodes, extract the lib_id (first string arg), then iterate `(pin ...)` children to build `PinDef` entries (name from `(name "...")`, number from `(number "...")`, pos/rotation from `(at x y rot)`, length from `(length ...)`, pin_type from the second atom)
- [ ] 3.4 Implement `fn find_symbol(symbols: &[SymbolDef], lib_id: &str) -> Option<&SymbolDef>` — match the library prefix separately (e.g., `"RP2040:RP2354a"` matches symbol `"RP2354a"` in file, library name is the KiCad library nickname, not part of the file)
- [ ] 3.5 Add unit tests with a small synthetic `.kicad_sym` string: verify pin extraction, multiple symbols, missing fields
- [ ] 3.6 Export `sym_parser` module from `crates/backends/kicad/src/lib.rs`

## 4. resolve_symbols pass

- [ ] 4.1 Add `fn resolve_symbols(design: &mut Design, lib_path: &str)` to `crates/backends/kicad/src/lib.rs`
- [ ] 4.2 Read the `.kicad_sym` file from `lib_path` (using `std::fs::read_to_string`), parse it once, cache the `Vec<SymbolDef>`
- [ ] 4.3 Iterate `design.components`; for each with `kicad_symbol == Some(sym_id)` and any pin with `pos == None`:
  - Find the matching `SymbolDef`
  - For each IR pin, find the matching `PinDef` by name (case-insensitive)
  - Set `pin.pos = Some((pin_def.pos.0, pin_def.pos.1))`, `pin.rotation = Some(pin_def.rotation)`
  - If the symbol is not found, add a `Diagnostic { severity: Warning, code: "SYM:NOT_FOUND", ... }`
  - If a pin is not matched, add a `Diagnostic { severity: Warning, code: "SYM:PIN_MISMATCH", ... }`
- [ ] 4.4 Re-export `resolve_symbols` from the crate root
- [ ] 4.5 Add unit tests: resolve a known symbol, resolve with missing symbol (check diagnostic), resolve with missing pin (check diagnostic), resolve when positions already set (no-op)

## 5. Schematic emitter changes

- [ ] 5.1 In `lib_pin_node`: when `pin.pos` is `Some((x,y))`, emit `(at x y rotation)` using `pin.rotation` (falling back to 180); otherwise keep algorithmic `(at 7.62 y_offset 180)`
- [ ] 5.2 In `lib_symbol_for_component`: when `kicad_symbol` is `Some(sym_id)`, use `sym_id` as the symbol name instead of `"copperleaf:{refdes}"`
- [ ] 5.3 In `symbol_instance_node`: when `kicad_symbol` is `Some(sym_id)`, use `sym_id` for `lib_id`
- [ ] 5.4 In `wire_node` and `label_node`: compute the pin tip position from `pin.pos` and `pin.rotation` (tip = pos + length * direction_vector(rotation)) when available; otherwise keep the algorithmic calculation
- [ ] 5.5 Update existing schematic tests to account for new fields (pins with `pos: None` should produce identical output to current)
- [ ] 5.6 Add tests: component with `kicad_symbol` and resolved pins produces `lib_id` referencing the real symbol and wires at real pin positions
- [ ] 5.7 Add a test with a small synthetic `.kicad_sym` string: `resolve_symbols` then `emit_schematic`, verify wire endpoints match symbol pin positions
- [ ] 5.8 Run `cargo test` to verify all schematic tests pass

## 6. Proc-macro crate: copperleaf-derive

- [ ] 6.1 Create `crates/derive/` directory with `Cargo.toml` (`[lib] proc-macro = true`, name `copperleaf-derive`, workspace fields)
- [ ] 6.2 Add `syn`, `quote`, `proc-macro2` to workspace `[workspace.dependencies]` with appropriate versions and features (`syn` with `derive` feature)
- [ ] 6.3 Add `copperleaf-derive` to `[workspace].members`
- [ ] 6.4 Create `crates/derive/src/lib.rs` with `#[proc_macro_derive(Component, attributes(component))]`
- [ ] 6.5 Parse the struct: require a field named `pins` of type `Vec<Pin>` (emit compile_error if missing)
- [ ] 6.6 Generate `impl Block for #name { fn pins(&self) -> &[Pin] { &self.pins } fn constraints(&self) -> Vec<Constraint> { vec![] } fn kicad_symbol(&self) -> Option<&str> { #symbol_attr } }`
- [ ] 6.7 Parse `#[component(symbol = "...")]` attribute via the `attributes(component)` parameter and the `syn::AttributeArgs`/`Meta` API
- [ ] 6.8 Add unit tests (proc-macro tests use `trybuild` or inline `compile_test` — for now, test via downstream crates)
- [ ] 6.9 Re-export from `copperleaf-edsl`: add `pub use copperleaf_derive::Component;` to `crates/edsl/src/lib.rs`, add `copperleaf-derive` dependency to `crates/edsl/Cargo.toml`

## 7. CLI integration

- [ ] 7.1 Add `--symbol-lib <path>` CLI flag to `copperleaf-cli` for the `export-sch` subcommand
- [ ] 7.2 When `--symbol-lib` is provided, call `resolve_symbols(&mut design, path)` before `emit_schematic`
- [ ] 7.3 Print any diagnostics produced by `resolve_symbols` to stderr
- [ ] 7.4 Add a test or manual verification step

## 8. Update parts crate with example

- [ ] 8.1 Add a test/example part in `crates/parts` or `crates/edsl` tests that uses `#[derive(Component)]` with `#[component(symbol = "...")]` to demonstrate the full flow
- [ ] 8.2 Include a small synthetic `.kicad_sym` test fixture file in `crates/backends/kicad/tests/` for integration tests
- [ ] 8.3 End-to-end test: define part with symbol → build design → resolve_symbols → emit_schematic → verify `lib_id` and wire coordinates match

## 9. Validate

- [ ] 9.1 Run `cargo build` across the workspace
- [ ] 9.2 Run `cargo test` across the workspace
- [ ] 9.3 Run `cargo clippy --all-targets -- -D warnings` if available
- [ ] 9.4 Run `cargo fmt --all`
- [ ] 9.5 Update `ARCHITECTURE.md` to mark `#[derive(Component)]` as implemented (§8)
- [ ] 9.6 Update `AGENTS.md` crate list to include `crates/derive`