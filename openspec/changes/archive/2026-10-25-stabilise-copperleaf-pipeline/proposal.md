# Proposal

## Why

A project-wide audit found that Copperleaf's architecture is sound, but the current implementation is not yet trustworthy enough for fabrication or unattended part generation. The workspace test run has 275 passing tests and 3 failures caused by incomplete goldens for four new parts. More importantly, physical connectivity is keyed by non-unique pin names, generated Rust trusts manifest strings, several KiCad emission paths produce semantically inconsistent output, and the active layout change claims routing work that the crate does not implement.

The project needs a correctness milestone before it accepts more parts, advances layout work, or publishes crates. Taking the breaking pin-identity change now is cheaper than preserving an ambiguous API into adoption.

## What Changes

- **Milestone 1 — trustworthy baseline.** Restore a green workspace, complete and review all missing goldens, make test failures report every missing artefact, enforce locked/reproducible CI, fix documentation failures, and require strict OpenSpec validation and a clean worktree.
- **Milestone 2 — board integrity. BREAKING.** Give every electrical pin a component-local physical identity derived from its pad number rather than its display name. Carry that identity through connections, net inference, ERC, and emitters. Permit repeated pin names. Add a central compiled-board validation pass that blocks emission when refdes, pad identities, net references, geometry, layers, or stackups are invalid. Correct electrical-ground selection and collision-free synthesised refdes allocation.
- **Milestone 3 — KiCad fabrication readiness.** Make schematic, netlist, and PCB emission agree per physical pad. Fully transform back-side footprints, preserve copper precision after DRC, use one layer vocabulary, generate globally unique deterministic IDs, separate footprint and 3D-model references, constrain output paths, and test preserved-board updates across deleted nets. Add version-pinned external KiCad loading, ERC, DRC, and schematic-parity acceptance.
- **Milestone 4 — secure part generation and honest release posture.** Treat datasheet and manifest input as untrusted. Parse quantities into typed values, escape all generated Rust, validate identifiers and manifests before every generation path, preserve `[layout]` data during updates, and constrain output paths. Reconcile the active layout change with its placement-only implementation, document actual capabilities and trust boundaries, and keep crates unpublished until a separate release decision.

This change deliberately does **not** implement autorouting. It establishes the correctness, validation, and external acceptance required before layout or release work can resume.

## Capabilities

### New Capabilities

- `board-integrity`: Component-local physical pin identity, duplicate-name support, central compiled-board invariants, and validation that blocks invalid boards before emission.
- `delivery-quality`: Reproducible verification gates, external KiCad acceptance, truthful capability documentation, and release readiness checks.

### Modified Capabilities

- `typed-pin-refs`: Pin references and handles identify physical pins even when multiple pins share a display name.
- `deterministic-ids`: Pin IDs follow physical identity; emitted UUIDs are unique for every addressable object and preserve byte reproducibility.
- `erc-and-synthesis`: Ground selection follows electrical meaning, and synthesised refdes allocation cannot collide with existing components.
- `parts-cli`: Manifest validation, typed generation, lossless serialisation, path containment, and safe handling of untrusted datasheet/LLM output.
- `kicad-backend`: Per-pad semantic parity, complete back-side transformation, canonical layers, full copper precision, unique IDs, correct model references, and safe preserved-board updates.

## Impact

- **`crates/core`**: `PinRef`, `PinHandle`, `PinId`, `Connection`, `BoardView`, canonical layer representation, and a public board-validation API.
- **`crates/compile`**: physical-identity grouping, net and ERC lookup, electrical-ground selection, collision-free synthesised refdes, and mandatory post-synthesis validation.
- **`crates/backend-kicad`**: schematic, netlist, PCB, footprint and model emitters; output-path containment; deterministic ID seeds; external acceptance fixtures.
- **`crates/part-codegen` and `crates/cli`**: typed quantity parsing, Rust literal and identifier validation, duplicate-name/pad validation, lossless manifest serialisation, secure vendor scaffolding, and LLM provenance handling.
- **`parts/*`**: regenerated constants and manifests; duplicate logical names are retained where they reflect the physical package.
- **Tests and CI**: reviewed goldens, semantic parity tests, malicious-input tests, deterministic ID checks, rustdoc and OpenSpec gates, dependency policy, golden-completeness checks, and a pinned `kicad-cli` acceptance job.
- **OpenSpec**: `layout-solving` is reconciled to its actual scope rather than archived with unsupported tasks checked. Stable capability purposes and malformed specs are repaired separately without changing current behaviour.
- **Compatibility**: physical pin references and generated part constants are a **BREAKING** API change. Datasheet/LLM workflows become stricter and may reject manifests that previously generated code.
- **Release**: workspace members remain explicitly unpublished until packaged-crate verification and the external acceptance suite pass.
