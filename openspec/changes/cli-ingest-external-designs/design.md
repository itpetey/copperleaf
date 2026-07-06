## Context

The CLI (`crates/cli/src/main.rs`) currently hardcodes `build_example_design()` in every subcommand. The function constructs a simple Buck + MCU design inline. There is no way to point the CLI at a design JSON file produced by an external project.

Consumer projects like halow-sta build their designs in Rust (`build_spi_reference_design()`) and run analysis inline via `run_analysis()`, which is ~166 lines of `println!` formatting. The CLI is completely unused.

Once `serialize-ir-connectivity` lands, designs can be serialized to JSON with full connectivity. The CLI can then load these files and run any analysis pass — making the CLI a general-purpose tool rather than a demo.

## Goals / Non-Goals

**Goals:**
- Every CLI subcommand accepts an optional `<design.json>` path.
- New `report` subcommand produces a human-readable design summary.
- New `emit` subcommand writes the example design as JSON (baseline for editing).
- A shared `report` function in `copperleaf-analysis` so library users can call it too.

**Non-Goals:**
- A full TUI or interactive mode.
- Design diffing or comparison (future work).
- Removing `build_example_design()` — it stays as the no-arg fallback and for `emit`.

## Decisions

### D1: Optional file path argument per subcommand

**Decision:** Subcommands use `cl <subcommand> [design.json]` — if a path is provided, load from file; otherwise use `build_example_design()`.

**Rationale:** Backwards-compatible with existing usage (`cl verify` still works). Simple arg parsing — no need for clap or a flag parser yet.

**Alternative considered:** `--file <path>` flag. Rejected for now because it adds a dependency or hand-rolled flag parsing for minimal benefit. Positional arg is simpler.

### D2: `report` subcommand backed by a library function

**Decision:** Add `pub fn report(design: &Design) -> String` to `copperleaf-analysis`. The CLI's `cmd_report()` calls it and prints. Library consumers can call it directly.

**Rationale:** The ~166 lines of reporting in halow-sta's `run_analysis()` is mostly generic (component list, net summary, ERC results, decoupling synthesis). Only the GPIO allocation table is project-specific. Moving the generic parts into copperleaf eliminates per-project reporting boilerplate.

**Output sections:**
1. Header (design title — derived from constraint count or a `Design::name` field if added later)
2. Graph stats (nodes, edges, components, nets, constraints)
3. Component list (grouped by refdes prefix)
4. Power & signal net summary
5. ERC results (overvoltage, NC pins, floating inputs)
6. Decoupling synthesis result

### D3: `emit` subcommand

**Decision:** `cl emit` runs `build_example_design()`, serializes to pretty JSON, writes to stdout. Useful for generating a baseline design file to edit manually or with patches.

**Rationale:** Gives users a starting point for the `apply` workflow and for testing CLI commands against a known design.

## Risks / Trade-offs

- **[Error handling]** Loading a malformed JSON file should produce a clear error, not a panic. → Use `expect()` with a descriptive message, or better, print an error and exit non-zero.
- **[Report completeness]** The built-in `report()` won't cover project-specific sections (e.g., SPI bus connectivity, GPIO allocation). → Acceptable: projects can call `report()` for the common parts and append their own sections.
- **[Dependency on serialize-ir-connectivity]** Without serializable connections, loading a design JSON loses all wiring. → This change should be implemented after `serialize-ir-connectivity`.
