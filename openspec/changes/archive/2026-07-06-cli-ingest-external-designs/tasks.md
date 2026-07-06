## 1. Shared design loader

- [x] 1.1 Add a `load_design(path: Option<&str>) -> Design` helper function in `crates/cli/src/main.rs` that reads from file if path is given, otherwise calls `build_example_design()`
- [x] 1.2 Update `cmd_verify`, `cmd_export`, `cmd_json`, `cmd_decouple` to accept `args: &[String]` and pass the first arg (if any) to `load_design()`
- [x] 1.3 Add error handling: if file read or JSON parse fails, print a clear error to stderr and exit non-zero

## 2. Report function in analysis crate

- [x] 2.1 Add `pub fn report(design: &Design) -> String` to `crates/analysis/src/lib.rs`
- [x] 2.2 Implement report sections: header (graph stats), component list (grouped by refdes prefix), power & signal net summary, ERC results (overvoltage + NC-pin checks), decoupling synthesis
- [x] 2.3 Re-export `report` through the `copperleaf` facade crate
- [x] 2.4 Add a unit test for `report()` that builds a small design and verifies the output string contains expected sections

## 3. Report and emit subcommands

- [x] 3.1 Add `cmd_report(args: &[String])` that calls `load_design()` and prints `report(&design)`
- [x] 3.2 Add `cmd_emit()` that calls `build_example_design()`, serializes to pretty JSON, and prints to stdout
- [x] 3.3 Update `main()` match to handle `"report"` and `"emit"` subcommands
- [x] 3.4 Update `usage()` to list all available subcommands including `report` and `emit`

## 4. Tests and validation

- [x] 4.1 Test that `cl emit` output can be deserialized as a `Design` (round-trip test)
- [x] 4.2 Test that `cl verify <file>` works on a JSON file produced by `cl emit` + patched connections
- [x] 4.3 Run `cargo test -p copperleaf-cli` and ensure all tests pass
- [x] 4.4 Run `cargo test` across the workspace
