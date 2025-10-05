# Repository Guidelines

## Project Structure & Module Organization
- Workspace root: `Cargo.toml` (workspace members).
- Crates:
  - `crates/core`: units (uom-backed), diagnostics, IDs.
  - `crates/ir`: IR (pins, nets, constraints, design) + serde.
  - `crates/analysis`: ERC and analysis passes.
  - `crates/backends/kicad`: minimal netlist/schematic emitter.
  - `crates/edsl`: builder/macros (stubs for now).
  - `crates/parts`: standard parts (e.g., Buck, MCU).
  - `crates/copperleaf`: public facade; re-exports subcrates.
  - `crates/cli`: `copperleaf` CLI (`verify`, `export`, `json`).

## Build, Test, and Development Commands
- Build all: `cargo build`
- Run CLI: `cl verify|export|json` (installed) or `cargo run -p copperleaf-cli -- verify|export|json` (from repo)
- Run example: `cargo run -p copperleaf --example basic`
- Tests: `cargo test` (unit tests across crates)
- Lint (if installed): `cargo clippy --all-targets -- -D warnings`
- Format: `cargo fmt --all`

## Coding Style & Naming Conventions
- Rust 2021; 4-space indentation; keep modules small and cohesive.
- Types/traits: `PascalCase`; functions/fields: `snake_case`; constants: `SCREAMING_SNAKE_CASE`.
- Prefer strong types over primitives (e.g., `Qty<Volt>`). Avoid `unwrap()` in library code.
- Keep public APIs in `crates/copperleaf`; keep backends/analysis behind crate boundaries.

## Testing Guidelines
- Use Rust’s built-in test framework (`#[test]`) per crate.
- Name tests after behavior, e.g., `overvoltage_detected`.
- Add snapshot-style tests for JSON IR/backends when outputs stabilize.
- Run `cargo test` before opening a PR.

## Commit & Pull Request Guidelines
- Messages: imperative mood, concise subject (≤72 chars), details in body.
- Conventional Commits encouraged (e.g., `feat(ir): add NetClass serde`).
- PRs: include a clear description, linked issues, and before/after notes for behavior or CLI changes. Add tests when feasible.

## Architecture Overview (Essentials)
- Core flow: EDSL/parts → IR → analysis (diagnostics/proposals) → backends.
- Units via `uom`; quantities serialized as `{ value, unit }` in JSON.
- Graph: `petgraph` placeholder; future work will wire connections and indexing.

## Security & Configuration Tips
- Avoid adding networked build steps. Keep third-party deps minimal and audited.
- Feature-gate experimental code; prefer additive, non-breaking changes.
