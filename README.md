# Copperleaf

A typed, constraint-driven Rust EDA library. Copperleaf describes components and
boards in Rust, compiles the design into an auditable net model, and emits a
complete KiCad project.

## Architecture

| Crate | Responsibility |
|-------|----------------|

| `crates/core` | Board builder, components, pins, pads, nets, stackup, units, ERC, and compiled-board validation. |
| `crates/compile` | Lowers a `Board` into a `CompiledBoard`, infers nets, runs ERC, validates, and synthesises decoupling capacitors. |
| `crates/layout` | Placement-only solver and layout design-rule checks. It does not route. |
| `crates/backend-kicad` | Emits `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, and `.net`, plus standalone `.kicad_sym`/`.kicad_mod` files. |
| `crates/part-codegen` | Generates typed Rust component crates from TOML manifests. |
| `crates/part-macro` | `build_component!` proc-macro entry point for generated parts. |
| `crates/cli` | `cl` parts-creation CLI (`new`, `update`, `generate`). |
| `parts/*` | Generated component libraries, one crate per vendor. |

The compilation path is:

```text
Board -> copperleaf_compile::run -> CompiledBoard -> Backend
```

`CompiledBoard` validation runs after synthesis and again before any backend
writes a file, so an invalid refdes, pin identity, net, dimension, layer, or
stackup cannot produce output.

## Prerequisites

- Rust stable with edition 2024 support (the committed `Cargo.lock` is used).
- Optional: `kicad-cli` 10.0.7 for external acceptance (`docs/acceptance.md`).
- Optional: `cargo-deny`, `cargo-shear`, and the OpenSpec CLI for the full
  verification workflow.

## Build and test

```sh
cargo build --workspace --all-targets --locked
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
```

## CLI examples

Create a part from a KiCad symbol:

```sh
cargo run -p copperleaf-cli -- new --symbol ic.kicad_sym \
  --lib-id RP2354A --crate raspberrypi
```

Create a part from a footprint or a directory containing both sources:

```sh
cargo run -p copperleaf-cli -- new --footprint ic.kicad_mod \
  --lib-id TEST --out test.toml
cargo run -p copperleaf-cli -- new --dir ./part_files/ \
  --lib-id TEST --crate testvendor
```

Update an existing vendor part from its symbol or footprint:

```sh
cargo run -p copperleaf-cli -- update --crate raspberrypi --lib-id RP2354A \
  --symbol ic.kicad_sym
```

Generate standalone KiCad library files from a manifest:

```sh
cargo run -p copperleaf-cli -- generate symbol parts/microchip/mcp73831t_2ati_ot.toml
cargo run -p copperleaf-cli -- generate footprint parts/microchip/mcp73831t_2ati_ot.toml
```

Datasheet mode requires the local `opencode` CLI and a configured provider:

```sh
cargo run -p copperleaf-cli -- new --datasheet datasheet.pdf --out part.toml
```

A small board can be built and emitted with the library API:

```rust
use copperleaf::{Backend, Board};
use copperleaf_backend_kicad::KiCad;

let mut board = Board::new("demo");
let u1 = board.add("U1", copperleaf_parts_wiznet::W5500::new());
let r1 = board.add(
    "R1",
    copperleaf_parts_passives::Resistor::new(
        33.0.kohm(),
        copperleaf_parts_passives::footprint::Package::M1608,
    ),
);
board.connect(u1.pin(copperleaf_parts_wiznet::W5500::GND), r1.pin(copperleaf_parts_passives::Resistor::PIN1))?;

let report = copperleaf_compile::run(board, &copperleaf_compile::CompileOptions::default())?;
KiCad::new().with_project_name("demo").emit("output", &report.board)?;
```

(Import `copperleaf::UnitExt` for `.kohm()`.)

## Supported capabilities

- Typed, compile-checked board construction with physical pin identity.
- Net inference, ERC, overvoltage and NC checks.
- Deterministic decoupling-capacitor synthesis and collision-free refdes allocation.
- Central compiled-board validation that blocks invalid emission.
- KiCad 10 project emission with schematic, netlist, PCB, footprints, and models.
- Deterministic UUIDs and byte-reproducible output.
- Placement-only layout solving with layout DRC.
- Manifest-to-Rust generation with typed quantities and escaped literals.
- Reviewed KiCad ERC/DRC/schematic-parity acceptance for a representative board.

## Unsupported capabilities

- Autorouting, track optimisation, and differential-pair or impedance tuning.
- Full fabrication sign-off, thermal simulation, and signal-integrity analysis.
- Automatic resolution of arbitrary external KiCad libraries at compile time.
- Release publication: every workspace package is explicitly unpublished.
- LLM-produced manifests are draft input only.

## Verification

The full gate set is documented in `docs/acceptance.md` for KiCad and below for
the workspace:

```sh
cargo build --workspace --all-targets --locked
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
openspec validate --specs --strict
cargo deny check
cargo shear
./scripts/kicad-acceptance.sh
```

CI runs the same locked build, format, Clippy, rustdoc, tests, and the pinned
KiCad acceptance job. Tests never bless snapshots in CI; creating or updating a
golden requires `COPPERLEAF_BLESS=1`.

## Trust boundaries

- Part manifests and generated Rust are treated as untrusted input.
- Strings are escaped through a Rust literal API and identifiers are validated.
- Datasheet text is sent to the configured model provider; the provider may
  retain it, and datasheets can contain prompt-injection text. Treat LLM output
  as a reviewable draft. Generated output is never compiled automatically.
- Backend output paths are contained beneath the destination directory; absolute
  paths, traversal, and symlink escapes are rejected before writing.
- Embedded 3D-model data is optional; empty payloads are rejected rather than
  written as zero-length files.

## Licence

Copperleaf is licensed under the Mozilla Public License 2.0. See
[`LICENCE`](LICENCE).
