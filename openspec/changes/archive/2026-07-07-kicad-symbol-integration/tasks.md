## 1. IR changes: Pin and ComponentRecord fields

- [x] 1.1 Add `pos: Option<(f64, f64)>` and `rotation: Option<f64>` to `Pin` in `crates/ir/src/lib.rs`
- [x] 1.2 Derive `Default` on `Pin` (or provide a sensible default impl)
- [x] 1.3 Update `Pin::new()` to initialize `pos: None, rotation: None`
- [x] 1.4 Add `kicad_symbol: Option<String>` to `ComponentRecord`
- [x] 1.5 Add `fn kicad_symbol(&self) -> Option<&str> { None }` to the `Block` trait as a default method
- [x] 1.6 Update `Design::add_component` to copy `kicad_symbol()` into the `ComponentRecord`
- [x] 1.7 Update all existing `Pin { ... }` struct literals in `crates/parts/src/lib.rs` to include `pos: None, rotation: None` (or use `..Default::default()`)
- [x] 1.8 Update test `Pin` literals in `crates/ir/src/lib.rs`, `crates/backends/kicad/src/schematic.rs`, `crates/analysis/src/lib.rs`, and `crates/edsl/src/lib.rs`
- [x] 1.9 Add unit tests: component without `kicad_symbol` gets `None`, component with `kicad_symbol()` override gets `Some(...)`
- [x] 1.10 Run `cargo build` and `cargo test` to verify IR compiles and all existing tests pass

## 2. S-expression parser in sexpr module

- [x] 2.1 Add `pub fn parse(input: &str) -> Result<Sexpr, ParseError>` to `crates/backends/kicad/src/sexpr.rs`
- [x] 2.2 Implement a tokenizer: atoms, quoted strings (with escape handling), `(`, `)`, and `#`-prefixed line comments
- [x] 2.3 Implement recursive-descent list/atom parsing into `Sexpr::List`/`Sexpr::Atom`
- [x] 2.4 Define `ParseError` type (unexpected EOF, unmatched paren, bad escape)
- [x] 2.5 Add unit tests: parse simple lists, nested lists, quoted strings, comments, whitespace tolerance, error cases
- [x] 2.6 Verify symmetry: `parse(&sexpr.to_string())` round-trips for representative KiCad fragments

## 3. Symbol library parser module

- [x] 3.1 Create `crates/backends/kicad/src/sym_parser.rs` with `SymbolDef` and `PinDef` structs
- [x] 3.2 Implement `fn parse_symbol_lib(input: &str) -> Result<Vec<SymbolDef>, ParseError>` using `sexpr::parse`
- [x] 3.3 Walk the parsed S-expression tree: find `(symbol ...)` nodes, extract the lib_id (first string arg), then iterate `(pin ...)` children to build `PinDef` entries (name from `(name "...")`, number from `(number "...")`, pos/rotation from `(at x y rot)`, length from `(length ...)`, pin_type from the second atom)
- [x] 3.4 Implement `fn find_symbol(symbols: &[SymbolDef], lib_id: &str) -> Option<&SymbolDef>` — match the library prefix separately (e.g., `"RP2040:RP2354a"` matches symbol `"RP2354a"` in file, library name is the KiCad library nickname, not part of the file)
- [x] 3.5 Add unit tests with a small synthetic `.kicad_sym` string: verify pin extraction, multiple symbols, missing fields
- [x] 3.6 Export `sym_parser` module from `crates/backends/kicad/src/lib.rs`

## 4. resolve_symbols pass

- [x] 4.1 Add `fn resolve_symbols(design: &mut Design, lib_path: &str)` to `crates/backends/kicad/src/lib.rs`
- [x] 4.2 Read the `.kicad_sym` file from `lib_path` (using `std::fs::read_to_string`), parse it once, cache the `Vec<SymbolDef>`
- [x] 4.3 Iterate `design.components`; for each with `kicad_symbol == Some(sym_id)` and any pin with `pos == None`:
  - Find the matching `SymbolDef`
  - For each IR pin, find the matching `PinDef` by name (case-insensitive)
  - Set `pin.pos = Some((pin_def.pos.0, pin_def.pos.1))`, `pin.rotation = Some(pin_def.rotation)`
  - If the symbol is not found, add a `Diagnostic { severity: Warning, code: "SYM:NOT_FOUND", ... }`
  - If a pin is not matched, add a `Diagnostic { severity: Warning, code: "SYM:PIN_MISMATCH", ... }`
- [x] 4.4 Re-export `resolve_symbols` from the crate root
- [x] 4.5 Add unit tests: resolve a known symbol, resolve with missing symbol (check diagnostic), resolve with missing pin (check diagnostic), resolve when positions already set (no-op)

## 5. Schematic emitter changes

- [x] 5.1 In `lib_pin_node`: when `pin.pos` is `Some((x,y))`, emit `(at x y rotation)` using `pin.rotation` (falling back to 180); otherwise keep algorithmic `(at 7.62 y_offset 180)`
- [x] 5.2 In `lib_symbol_for_component`: when `kicad_symbol` is `Some(sym_id)`, use `sym_id` as the symbol name instead of `"copperleaf:{refdes}"`
- [x] 5.3 In `symbol_instance_node`: when `kicad_symbol` is `Some(sym_id)`, use `sym_id` for `lib_id`
- [x] 5.4 In `wire_node` and `label_node`: compute the pin tip position from `pin.pos` and `pin.rotation` (tip = pos + length * direction_vector(rotation)) when available; otherwise keep the algorithmic calculation
- [x] 5.5 Update existing schematic tests to account for new fields (pins with `pos: None` should produce identical output to current)
- [x] 5.6 Add tests: component with `kicad_symbol` and resolved pins produces `lib_id` referencing the real symbol and wires at real pin positions
- [x] 5.7 Add a test with a small synthetic `.kicad_sym` string: `resolve_symbols` then `emit_schematic`, verify wire endpoints match symbol pin positions
- [x] 5.8 Run `cargo test` to verify all schematic tests pass

## 6. Proc-macro crate: copperleaf-derive

- [x] 6.1 Create `crates/derive/` directory with `Cargo.toml` (`[lib] proc-macro = true`, name `copperleaf-derive`, workspace fields)
- [x] 6.2 Add `syn`, `quote`, `proc-macro2` to workspace `[workspace.dependencies]` with appropriate versions and features (`syn` with `derive` feature)
- [x] 6.3 Add `copperleaf-derive` to `[workspace].members`
- [x] 6.4 Create `crates/derive/src/lib.rs` with `#[proc_macro_derive(Component, attributes(component))]`
- [x] 6.5 Parse the struct: require a field named `pins` of type `Vec<Pin>` (emit compile_error if missing)
- [x] 6.6 Generate `impl Block for #name { fn pins(&self) -> &[Pin] { &self.pins } fn constraints(&self) -> Vec<Constraint> { vec![] } fn kicad_symbol(&self) -> Option<&str> { #symbol_attr } }`
- [x] 6.7 Parse `#[component(symbol = "...")]` attribute via the `attributes(component)` parameter and the `syn::AttributeArgs`/`Meta` API
- [x] 6.8 Add unit tests (proc-macro tests use `trybuild` or inline `compile_test` — for now, test via downstream crates)
- [x] 6.9 Re-export from `copperleaf-edsl`: add `pub use copperleaf_derive::Component;` to `crates/edsl/src/lib.rs`, add `copperleaf-derive` dependency to `crates/edsl/Cargo.toml`

## 7. CLI integration

- [x] 7.1 Add `--symbol-lib <path>` CLI flag to `copperleaf-cli` for the `export-sch` subcommand
- [x] 7.2 When `--symbol-lib` is provided, call `resolve_symbols(&mut design, path)` before `emit_schematic`
- [x] 7.3 Print any diagnostics produced by `resolve_symbols` to stderr
- [x] 7.4 Add a test or manual verification step

## 8. Update parts crate with example

- [x] 8.1 Add a test/example part in `crates/parts` or `crates/edsl` tests that uses `#[derive(Component)]` with `#[component(symbol = "...")]` to demonstrate the full flow
- [x] 8.2 Include a small synthetic `.kicad_sym` test fixture file in `crates/backends/kicad/tests/` for integration tests
- [x] 8.3 End-to-end test: define part with symbol → build design → resolve_symbols → emit_schematic → verify `lib_id` and wire coordinates match

## 9. Validate

- [x] 9.1 Run `cargo build` across the workspace
- [x] 9.2 Run `cargo test` across the workspace
- [x] 9.3 Run `cargo clippy --all-targets -- -D warnings` if available
- [x] 9.4 Run `cargo fmt --all`
- [x] 9.5 Update `ARCHITECTURE.md` to mark `#[derive(Component)]` as implemented (§8)
- [x] 9.6 Update `AGENTS.md` crate list to include `crates/derive`