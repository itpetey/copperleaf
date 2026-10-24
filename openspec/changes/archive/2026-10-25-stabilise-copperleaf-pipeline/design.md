# Design

## Context

See `proposal.md` for motivation and the seven delta specs for the behavioural contract. The current codebase is a 15-crate Rust workspace using edition 2024. Compilation is `Board -> copperleaf_compile::run -> CompiledBoard -> Backend`, with generated part crates produced from TOML manifests.

The constraints shaping this design are:

- Existing goldens are extensive but characterise current defects; they cannot be accepted as the correctness oracle.
- `PinRef` is already a zero-cost static string, so physical identity can replace its value without changing most authored call sites.
- `layout-solving` is an active, invalid change. This change does not implement routing or compete with that capability; it requires the active change to be rebaselined honestly.
- KiCad 10.0.7 is available locally and successfully loads current output, so external acceptance is feasible without replacing the backend.
- The workspace is pre-release. This is the correct point for the breaking physical-identity migration.

## Goals / Non-Goals

**Goals:**

- Establish a green, reproducible baseline before behavioural work.
- Make physical identity the single source of electrical connectivity.
- Put validation at the compiler/backend trust boundary.
- Make emitted views agree and prove it with KiCad itself.
- Make manifest-to-Rust generation safe to consume from untrusted data.
- Leave layout and publishing as explicit, separately authorised decisions.

**Non-Goals:**

- Implementing autorouting, improving placement quality, or fixing the layout algorithm.
- Designing a second KiCad backend or replacing S-expressions with an external KiCad library.
- Supporting differential pairs, impedance tuning, or full fabrication sign-off.
- Publishing crates in this change.
- Making LLM-generated parts authoritative. They remain draft input requiring review.

## Decisions

### D1: Canonical pin identity is the resolved pad number

Each electrical pin gets a component-local canonical key from its resolved pad number, using the same precedence already used by pad resolution: explicit pad number, then pin number, then 1-based insertion index. Core SHALL own this resolution so compilation and every emitter derive the number identically. A missing explicit number emits a warning during validation; the index fallback keeps hand-written components usable.

`PinRef` remains `PinRef(pub &'static str)`; only its value becomes the canonical key. Generated constants keep a readable name when unique (`VDD = PinRef("28")`) and append a sanitised physical suffix when names repeat (`AVDD_4 = PinRef("4")`, `AVDD_8 = PinRef("8")`). If suffix collision occurs after sanitisation, generation appends an ordinal. This retains type-safe authoring while making every physical pin independently addressable.

`Connection` stores a `PinId` rather than a display-name string. Compilation resolves each `PinHandle` key to exactly one component pin once, then builds union-find nodes from `(component_index, pin_index)`. `BoardView` exposes the same index mapping. Emitters resolve the ID back to that index for name, number, and geometry.

Alternative considered: keep names and add an occurrence counter to handles. Rejected because every lookup would still need disambiguation and declaration-order changes would silently retarget connections. Alternative: use `PinId` directly in authored constants. Rejected because generated IDs are opaque and would make board code unreadable.

### D2: Validation is one core-owned invariant pass

Add a public compiled-board validation API returning `Vec<Diagnostic>` with `BOARD:` codes. It checks, in one pass:

- non-empty unique refdes and unique `PinId`/canonical number per component;
- finite, positive board dimensions, design rules, pad dimensions, track widths, via drill/diameter, and zone geometry;
- valid component, pin, net, placement, layer, and stackup references;
- unique net names and consistent connection-to-net ownership;
- stackup starts/ends with copper and alternates copper/dielectric;
- layout placement uniqueness and finite coordinates;
- layout geometry within the configured copper-layer count.

Compilation runs validation after synthesis and before returning. Backends run the same validation before writing because `CompiledBoard` is publicly constructible in tests and downstream code. Validation is not part of `Board` construction because intermediate builder states are intentionally incomplete.

Alternative considered: validate in each emitter. Rejected because invariants then diverge and a new backend becomes a new source of defects.

### D3: Layer references are zero-based copper ordinals

Introduce a `CopperLayer(pub usize)` newtype in core. Layout tracks, vias, zones, and plane directives use it. `Stackup` validates ordinals against its copper count. The KiCad backend alone maps ordinal `0` to `F.Cu`, the last ordinal to `B.Cu`, and intermediate ordinals to `In<n>.Cu`.

This removes the current mix of KiCad layer numbers and ordinals. The active `layout-solving` delta's `Plane { layer: 31 }` example must be rebaselined to ordinal `1` on a two-layer board; no stable layout capability is being modified by this change.

Alternative considered: standardise on KiCad layer numbers in core. Rejected because core is backend-neutral and inner-layer numbering is a format concern.

### D4: Back-side emission uses one geometric transform

Footprint emission computes a side transform once and applies it to pads, fabrication/silkscreen/courtyard graphics, visible text, and the embedded model reference. Layer names map symmetrically: `F.Cu`/`B.Cu`, `F.Mask`/`B.Mask`, `F.Paste`/`B.Paste`, `F.SilkS`/`B.SilkS`, `F.Fab`/`B.Fab`, and `F.CrtYd`/`B.CrtYd`; wildcard layers remain unchanged. KiCad's own round-trip accepted output is the deciding test for orientation, not hand-derived coordinates.

Copper uses a dedicated formatter at six decimal millimetres (nanometre resolution), with no 0.01 mm quantisation. The courtyard-friendly `format_grid_float` remains limited to footprint graphics.

### D5: Emitted-object UUIDs are seeded by addressable identity

Extend every deterministic UUID seed to include the complete discriminating path:

- footprint property: refdes + property name + value + property ordinal;
- segment: net + canonical layer + path ordinal + segment ordinal;
- via: net + start layer + end layer + via ordinal;
- zone: net + canonical layer + zone ordinal;
- labels/wires: existing net/name/coordinate seed plus object ordinal.

A test parses every UUID from each emitted project and asserts global uniqueness. Seeds remain stable across processes, preserving byte reproducibility.

### D6: Untrusted manifest data is parsed, never interpolated as Rust

Manifest scalar quantities move to reusable typed definitions: a numeric value plus a closed unit enum, with a backward-compatible string parser for existing `"0.2mm"`, `"100nF"`, and similar values. Constraint and layout definitions use these typed values. Codegen renders typed quantities through constructors such as `.mm()`; it never accepts or copies a Rust expression.

All generated string literals pass through `syn::LitStr`, and all generated identifiers pass `syn::Ident`. Template fields contain either already-safe token text or plain display text, not raw user input at triple-brace substitution points. Manifest structs use `deny_unknown_fields`. Validation is called from the CLI, codegen entry points, and the build macro; invalid input returns diagnostics without producing a partial file.

Manifest serialisation switches to serialising the complete `Manifest` value, including `[layout]`, rather than manually listing selected sections. Comment injection can remain a post-pass, but semantic round-trip is the contract. Existing semantic round-trip tests expand to compare every field.

Alternative considered: HTML-escape Mustache substitutions. Rejected because HTML escaping is incorrect for Rust and still permits identifier and expression injection.

### D7: Filesystem output uses one containment rule

Add a small shared path utility that accepts a trusted root and one untrusted path component. It rejects NUL, absolute components, `.`/`..`, path separators, and empty names; for existing targets it canonicalises and checks the root; for new targets it canonicalises the deepest existing parent before joining. The CLI and KiCad backend use it for project names, output filenames, local footprint names, model names, and vendor directories. Failure occurs before any file is written.

### D8: Ground selection and refdes allocation are deterministic

Ground selection prefers a connected net with at least one ground-role pin, then a zero-volt power net in net-index order, and finally creates one synthetic ground net. The name `GND` alone is never sufficient. If a non-ground net is named `GND`, synthesis emits a warning.

Synthesis scans existing refdes with the capacitor prefix, parses numeric suffixes, and starts at `max + 1`, skipping occupied names. It emits a `SynthCap` entry for every synthesised component, satisfying the existing compile-report contract.

### D9: Acceptance is a reviewed baseline, not a zero-findings claim

A dedicated representative W5500 fixture exercises repeated pin names, power and ground, mechanical pads, and back-side placement. `kicad-cli` 10.0.7 (or a pinned 10.x container digest after verification) loads all project files and produces JSON ERC/DRC/parity reports. Reviewed findings live in a checked-in baseline; any new finding fails acceptance. This avoids pretending synthetic goldens or intentionally incomplete fixtures are fabrication-ready.

Golden tests continue to pin bytes, but acquire semantic assertions for pad/net parity, warning sets, UUID uniqueness, and compiler-validation success. Missing goldens are collected across all manifests before failing. `COPPERLEAF_BLESS=1` is the only blessing value.

### D10: Layout claims are rebaselined; routing is deferred

The active `layout-solving` change is edited during implementation to reflect placement-only behaviour, unsupported directives as diagnostics, and the absence of Topola/routing. Its archive remains blocked until the spec and implementation agree. No autorouter is added by this change; a future change can choose Topola, a simpler router, or manual routing after this correctness baseline.

### D11: Release remains disabled by default

Every workspace package gets `publish = false`. Internal path dependencies remain path-only until a release decision, because adding registry versions prematurely implies publication. A release change must add complete package metadata, asset provenance, versioned internal dependencies, and unpacked-package build tests. `cargo package` remains useful for contents review; publication is impossible until the flags are deliberately removed.

### D12: One verification command drives CI

Add a repository script that runs: locked all-target build, format check, Clippy and rustdoc with denied warnings, all workspace tests, strict OpenSpec validation, dependency policy, golden completeness/dirty-tree checks, and KiCad acceptance. CI invokes the same script in separate fast and KiCad-aware jobs if image setup warrants it, but failures and ordering remain defined in one place. Third-party GitHub Actions are pinned to commit SHAs with read-only permissions.

## Risks / Trade-offs

- **Physical identity migration touches nearly every golden and test** → Land it atomically after baseline restoration; review semantic diffs before blessing; preserve readable constants for unique names.
- **Resolved pad numbers can disagree between `Pin.number` and `Pad.number`** → Validation fails on disagreement instead of silently choosing one.
- **Generated constant suffixes can be ugly for non-numeric pad names** → Sanitise deterministically and add ordinals only on collision; expose the readable pin name in docs and diagnostics.
- **Central validation can initially reject many existing boards** → Run it in report-only mode during development, review every diagnostic, then make it blocking before merge; do not weaken checks to preserve a golden.
- **Back-side orientation is easy to get subtly wrong** → Use one transform for all geometry, compare KiCad round-trip output, and require external load/DRC acceptance.
- **KiCad CLI version and CI image availability are environment-sensitive** → Pin version/digest, keep the script runnable locally with a configurable binary, and record expected reports rather than hiding findings.
- **Strict manifest parsing breaks existing manifests** → Migrate all 27 manifests through a checked migration and report exact field paths; releases remain disabled until complete.
- **More CI stages increase feedback time** → Keep fast Rust/OpenSpec checks separate from the containerised KiCad job while retaining one aggregate verification command.
- **OpenSpec cleanup expands beyond code** → Limit it to truthfulness and structural validity required by `delivery-quality`; behavioural layout changes remain a separate change.
- **The PDF stack carries an unmaintained transitive dependency** → Record a scoped, review-dated `RUSTSEC-2026-0192` exception in dependency policy; replacing the PDF stack remains a later maintenance change.

## Migration Plan

1. **Baseline:** add the verification script, collect all missing goldens without blessing, review all 12 new artefacts, repair rustdoc/CI/OpenSpec structural failures, and record the current KiCad acceptance baseline.
2. **Identity:** introduce canonical pin resolution and `CopperLayer` in core, switch compilation and `BoardView` to pin indices, change `Connection` to `PinId`, regenerate part constants, and migrate all call sites/tests in one change. No compatibility shim.
3. **Validation:** add the central validation pass in report-only mode, resolve all findings on shipped parts and golden boards, then make it blocking in compile and backend entry points.
4. **Emission:** fix per-pad net parity, back-side transforms, copper precision, UUID seeds, model/footprint separation, path containment, and preserved-update net clearing. Re-bless only after semantic and KiCad acceptance pass.
5. **Generation:** migrate manifests to typed quantities, add literal/identifier validation, complete serialisation, LLM attachment/provenance handling, and rebuild-safe vendor scaffolding.
6. **Close-out:** reconcile `layout-solving`, repair stable specs, expand README/CONTRIBUTING with supported capabilities and trust boundaries, set `publish = false`, and run the complete verification command.

Rollback is per milestone: baseline changes are additive; identity is one atomic commit series; validation can temporarily report-only; emission changes are guarded by reviewed goldens; generation migration can revert to the prior manifest parser while keeping stricter output paths. No milestone publishes or archives a change.

## Open Questions

- Which additional board project, beyond the representative W5500 acceptance fixture, should join external acceptance once a real downstream design is available? This affects fixture coverage only, not the approach or task sequence.
