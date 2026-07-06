## Why

Every CLI subcommand (`verify`, `export`, `json`, `decouple`) calls `build_example_design()` — a hardcoded Buck+MCU design. A project like halow-sta that defines its own parts and builds its own design cannot use the CLI at all. The `apply` command is the only one that reads a file, and it works only because it replays patch ops on a deserialized (empty-graph) design. Once connectivity is serializable (see `serialize-ir-connectivity` change), the CLI can load complete designs from JSON and run any analysis pass on them. This makes the CLI a real tool for real projects, not just a demo.

## What Changes

- All CLI subcommands (`verify`, `export`, `json`, `decouple`) accept an optional `<design.json>` path argument. When provided, the design is loaded from JSON instead of using `build_example_design()`.
- New `report` subcommand: loads a design and prints a human-readable summary (components, nets, connectivity, ERC, decoupling). This replaces the ~166 lines of hand-rolled `println!` reporting that every consumer project currently writes.
- New `emit` subcommand: takes no args, runs `build_example_design()`, and writes JSON to stdout (useful for generating a baseline design JSON to edit).
- The `apply` command already loads from JSON; it continues to work and now benefits from the graph being rebuilt on deserialization.

## Capabilities

### New Capabilities
- `cli-external-design`: CLI subcommands load and operate on external design JSON files, plus a `report` subcommand for human-readable summaries.

### Modified Capabilities

## Impact

- **`crates/cli/src/main.rs`**: Refactor all subcommands to accept optional file path argument. Add `cmd_report()` and `cmd_emit()`. Factor out a `load_design(path: Option<&str>) -> Design` helper.
- **`crates/analysis/src/lib.rs`**: Add a `report` module or function that produces structured text output from a `&Design`. This is shared between the CLI and library consumers.
- **No breaking changes** to the library API — this is additive.
- Depends on `serialize-ir-connectivity` for correct graph rebuilding on deserialization.
