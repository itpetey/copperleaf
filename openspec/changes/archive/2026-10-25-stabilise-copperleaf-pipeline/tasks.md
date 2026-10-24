# Tasks

## 1. Trustworthy baseline

- [x] 1.1 Create the twelve missing golden artefacts for `tlc5946pwp`, `tps23753apwr`, `w55rp20`, and `749119250`, inspecting each symbol/footprint/render diff for valid pin numbers and geometry before accepting it; verify both goldens suites pass without blessing.
- [x] 1.2 Change dynamic golden harnesses to enumerate every manifest and report all missing files before failing, require `COPPERLEAF_BLESS=1` exactly, and assert board warnings plus semantic invariants; verify tests cover all-missing and `COPPERLEAF_BLESS=0` cases.
- [x] 1.3 Fix the three rustdoc warnings and any newly exposed documentation failures; verify `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` passes.
- [x] 1.4 Update CI to use `--locked`, pin third-party actions to commit SHAs, set `permissions: contents: read`, and fail on a dirty worktree; verify the workflow matches a local locked build/format/Clippy/test run.
- [x] 1.5 Add dependency-policy configuration for compatible licences, advisories, duplicate bans, and sources, plus `cargo-shear`; document a scoped exception for `RUSTSEC-2026-0192` with a follow-up note, remove the unused Wurth dependency, and verify `cargo deny check` and `cargo shear` pass.
- [x] 1.6 Repair stable OpenSpec structure: add the missing `component-metadata` and `pad-model` sections, replace archive placeholders with real purposes, and remove delta-only headers from stable specs; verify `openspec validate --specs --strict` passes.
- [x] 1.7 Add a KiCad acceptance harness that loads the current representative project, captures reviewed ERC/DRC/parity JSON, and fails on differences; verify it runs with local KiCad 10.0.7 and records current findings without claiming a clean board.

## 2. Physical identity and board validation

- [x] 2.1 Add core-owned canonical pin resolution using explicit pad number, then pin number, then insertion ordinal; include diagnostics for missing numbers, conflicting `Pin.number`/`Pad.number`, and duplicate component-local identities; verify unit tests cover each precedence, repeated names, and duplicate rejection.
- [x] 2.2 Seed `PinId` from refdes plus canonical physical identity and update generated pin constants so unique names remain readable while repeated names gain sanitised physical suffixes; verify codegen tests cover unique names, repeated names, non-numeric pad numbers, and suffix collisions.
- [x] 2.3 Replace name-keyed `Connection` storage, net grouping, `BoardView`, ERC lookup, and backend pin lookup with `PinId`/component-local pin indices; verify a W5500 fixture connects two same-named ground pads independently and leaves the unconnected pad unassigned.
- [x] 2.4 Regenerate every affected part crate and board fixture after the identity migration, reviewing constant and connectivity diffs rather than blindly blessing; verify workspace build, unit/integration tests, and all current goldens pass.
- [x] 2.5 Introduce `CopperLayer` as a zero-based copper ordinal, thread it through layout geometry and plane directives, validate it against stackup copper count, and map it only in the KiCad backend; verify two-layer, four-layer, and out-of-range tests.
- [x] 2.6 Implement the central compiled-board validation pass covering refdes, pin identity, net ownership, finite dimensions, geometry, layer references, stackups, placements, tracks, vias, and zones, initially in report-only mode; verify one test per invariant plus a multi-error report.
- [x] 2.7 Resolve every validation finding on shipped parts and fixtures, then make validation block compilation and backend emission; verify invalid manually constructed boards cannot produce output and valid boards compile unchanged.
- [x] 2.8 Correct decoupling-ground selection and capacitor refdes allocation, populate `CompileSummary.caps_synthesised`, and preserve compile warning/report behaviour; verify zero-volt selection, misleading `GND` warning, synthetic ground, and occupied-refdes scenarios.

## 3. KiCad emission and external acceptance

- [x] 3.1 Make schematic, netlist, and PCB emission agree on every physical pad association and unconnected state; verify a W5500 regression asserts every repeated-name pad across all three outputs.
- [x] 3.2 Apply one back-side footprint transform to pads, copper/mask/paste layers, fab/silkscreen/courtyard graphics, text, and model references; verify no front-side layers remain and a KiCad-loaded back-side footprint round-trips correctly.
- [x] 3.3 Emit copper coordinates and widths at six-decimal millimetre precision after DRC, add canonical layer mapping, and expand UUID seeds with object siblings/ordinals; verify sub-0.01 mm geometry survives emission and every UUID in each project is unique.
- [x] 3.4 Separate footprint references from 3D-model references in passive metadata, reject empty model payloads, and prevent zero-length model files; verify passive goldens use `Library:Footprint` identifiers and a separate model path.
- [x] 3.5 Add the shared path-containment utility and apply it to backend project, footprint, and model filenames before writing; verify absolute, traversal, and symlink-escape tests write nothing outside the destination.
- [x] 3.6 Clear deleted nets from preserved PCB pads, prevent reused net codes from aliasing old copper, and preserve copper for renamed nets; verify deleted, renamed, and code-reuse regression tests.
- [x] 3.7 Build the representative W5500 acceptance fixture with duplicate names, planes, mechanical pads, and back-side placement; record reviewed KiCad ERC/DRC/parity baselines and add the pinned KiCad acceptance job; verify the job fails when an expected finding is added or removed.
- [x] 3.8 Document acceptance setup, baseline review, supported KiCad version, and known limitations in `docs/acceptance.md`; verify the documented command runs locally and in CI.

## 4. Secure manifest and part generation

- [x] 4.1 Replace string-valued manifest quantities with typed value/unit definitions while accepting existing suffix strings, migrate all 27 part manifests, and render only known quantity constructors; verify parser, invalid-expression, and full-workspace migration tests.
- [x] 4.2 Escape every generated Rust string through a Rust literal API and validate struct, constant, and field identifiers before rendering; verify malicious quotes, newlines, comments, invalid identifiers, and raw Rust expressions cannot enter generated source.
- [x] 4.3 Reject unknown manifest fields and run the same validation from CLI generation, codegen entry points, and the build macro without partial output; verify each entry point rejects invalid input with field-addressed diagnostics.
- [x] 4.4 Switch manifest serialisation to a complete semantic round-trip including `[layout]`, metadata, constraints, and mechanical pads; verify all part manifests and a layout-bearing fixture preserve every field through serialise/deserialise.
- [x] 4.5 Pass named attachments to the datasheet workflow, use `datasheet.txt` for new-part prompts, preserve model payloads, and prevent automatic compilation of generated output; verify new/update prompt tests and document provider-disclosure/prompt-injection risk in CLI help.
- [x] 4.6 Apply path containment to CLI output, vendor, footprint, and model paths derived from manifests; verify traversal and absolute-path attempts fail before writing.
- [x] 4.7 Generate a package-local TOML rebuild script and update workspace membership with TOML-aware editing, including empty member arrays and Wurth migration; verify scaffolded and existing crates rebuild after TOML-only edits.
- [x] 4.8 Regenerate all codegen, symbol, footprint, and board goldens after the typed-manifest migration; verify diffs contain only reviewed semantic changes and all workspace tests pass.

## 5. Documentation, specifications, and release posture

- [x] 5.1 Expand `README.md` with architecture, prerequisites, CLI/example commands, supported and unsupported capabilities, verification workflow, trust boundaries, and licence; verify every documented command against the workspace.
- [x] 5.2 Add `CONTRIBUTING.md` covering part creation, generated-code review, golden blessing, breaking pin identity, KiCad acceptance, third-party asset provenance, and OpenSpec lifecycle; verify links and commands.
- [x] 5.3 Remove developer-local `/Users/...` paths from manifests and goldens, replace empty model payloads, and add a third-party asset provenance register; verify no tracked manifest contains a local absolute path and every embedded model has recorded provenance.
- [x] 5.4 Set `publish = false` on every workspace package and add missing package metadata where it remains meaningful; verify `cargo metadata` shows no publishable package and an attempted publish is refused.
- [x] 5.5 Rebaseline `layout-solving` to placement-only reality: reopen unsupported tasks, remove Topola/routing completion claims, add unsupported-directive diagnostics to its plan, and resolve its delta validation errors; verify `openspec validate layout-solving --strict` passes and archive remains blocked appropriately.
- [x] 5.6 Repair remaining stable-spec purposes and content that contradict implemented APIs or capabilities; verify `openspec validate --all --strict` passes.
- [x] 5.7 Add one aggregate verification script covering locked build, format, Clippy, rustdoc, tests, strict OpenSpec, dependency policy, golden cleanliness, determinism, and KiCad acceptance, and invoke it from CI; verify the script fails on an injected check failure and passes on a clean worktree.
- [x] 5.8 Run the full verification command and inspect the final diff for uncommitted generated files, stale goldens, or unrecorded acceptance changes; verify every required artefact is present and all checks pass.

## Workflow follow-up

- Archive this change only after review, a clean full verification run, and maintenance decisions for any accepted dependency advisory.
- Sync the delta specs into main specs, then verify the archived change and stable specs.
