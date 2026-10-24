# Contributing to Copperleaf

## Part creation

Every part lives in `parts/<vendor>/<part>.toml` and is compiled into a typed
crate member by `build_component!`. Create a manifest from a KiCad symbol,
footprint, directory, or datasheet:

```sh
cargo run -p copperleaf-cli -- new --symbol ic.kicad_sym --lib-id PART --crate vendor
cargo run -p copperleaf-cli -- update --crate vendor --lib-id PART --footprint ic.kicad_mod
```

Guidelines:

- Every electrical pin must have a distinct physical pad number. Display names
  may repeat; generated constants gain a physical suffix (`AVDD_4`, `AVDD_8`).
- Give every power pin voltage/current limits. Empty placeholders are drafts,
  not finished parts.
- Use typed quantity strings (`"100nF"`, `"0.2mm"`, `"85C"`). Raw Rust
  expressions are rejected.
- Do not commit developer-local paths (for example `/Users/...`). Record model
  provenance and embed model data only when it is redistributable.
- Run the locked build and tests before opening a pull request.

## Generated code review

Generated constants, builders, footprints, and board files are reviewed like
hand-written code. Do not accept a diff merely because a test blessed it.
Check:

- pin numbers, names, roles, and pad geometry against the datasheet;
- emitted layer names, back-side geometry, and model references;
- net connectivity and netlist nodes for repeated-name pads;
- deterministic UUID changes are expected only when the addressable object
  changed.

Generated Rust is written through escaped Rust literals and validated
identifiers. Never insert raw manifest text into a template.

## Golden files

Goldens live under:

- `crates/part-codegen/tests/golden/render/`
- `crates/cli/tests/golden/generate/`
- `crates/backend-kicad/tests/golden/board/`

Create or update them only after reviewing the diff:

```sh
COPPERLEAF_BLESS=1 cargo test -p copperleaf-part-codegen --test golden_render
COPPERLEAF_BLESS=1 cargo test -p copperleaf-cli --test golden_generate
COPPERLEAF_BLESS=1 cargo test -p copperleaf-backend-kicad --test golden_board
```

`COPPERLEAF_BLESS` must equal exactly `1`. CI never blesses and fails if a test
modifies tracked files. A missing golden is a failure that lists every missing
path, not an invitation to bless blindly.

## Breaking pin identity changes

Pin identity is physical, not a display name. A change that alters pad numbers,
canonical identities, generated constants, or `Connection` storage is a
breaking API change. Land it atomically:

1. update core resolution and codegen;
2. migrate compilation, `BoardView`, ERC, and every backend;
3. regenerate and review every affected part constant and board golden;
4. verify repeated-name pads independently across schematic, netlist, and PCB.

Do not add a compatibility shim that silently falls back to display names.

## KiCad acceptance

Acceptance is a reviewed baseline, not a zero-findings claim. Run:

```sh
./scripts/kicad-acceptance.sh
```

If findings change, inspect the named diff and fix unintended emitter changes.
Update the baseline only after review:

```sh
KICAD_ACCEPTANCE_BLESS=1 ./scripts/kicad-acceptance.sh
```

The pinned KiCad version and current known limitations are documented in
[`docs/acceptance.md`](docs/acceptance.md). CI runs the same script in a pinned
container.

## Third-party assets and provenance

Before adding a symbol, footprint, 3D model, or datasheet:

- confirm the licence permits redistribution;
- record the source and licence in the part manifest or the asset register;
- do not embed binaries or links whose provenance cannot be stated;
- remove developer-local absolute paths.

## OpenSpec lifecycle

Specs and changes live under `openspec/`. A behaviour change needs a change
with proposal, specs, design, and tasks. Implement tasks incrementally, marking
each task complete only when its specified behaviour is implemented and tested.

```sh
openspec list --json
openspec status --change "<name>" --json
openspec instructions apply --change "<name>" --json
openspec validate --specs --strict
```

Archive only after review, a clean full verification run, and syncing the delta
specs into main specs. Keep documentation and checked tasks truthful; an active
change must not claim unimplemented capability.

## Verification

Run the complete gate set before submitting:

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
