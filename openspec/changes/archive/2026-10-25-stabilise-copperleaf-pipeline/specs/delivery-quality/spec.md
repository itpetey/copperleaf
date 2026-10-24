# Spec Delta

## Purpose

Defines reproducible verification, external KiCad acceptance, truthful capability documentation, and release gates so repository health and advertised behaviour can be trusted.

## ADDED Requirements

### Requirement: Verification gates are reproducible

The repository SHALL provide one documented verification command that builds every target with the committed lockfile, checks formatting, denies Clippy and rustdoc warnings, runs all tests, validates OpenSpec strictly, and checks dependency policy. CI SHALL run the same command and fail when any stage fails.

#### Scenario: Warning fails verification
- **WHEN** a Rustdoc warning is introduced
- **THEN** the verification command exits non-zero
- **AND** CI reports the failing stage

#### Scenario: Lockfile drift fails verification
- **WHEN** the committed `Cargo.lock` is stale for the workspace
- **THEN** the locked build fails rather than silently resolving new versions

### Requirement: Golden suites have complete and reviewed coverage

Dynamic golden suites SHALL enumerate every part manifest and report every missing golden in one failure. CI SHALL NOT bless snapshots and SHALL fail if tests modify tracked files. Snapshot creation SHALL require `COPPERLEAF_BLESS` to equal exactly `1`. Board snapshots SHALL assert compiler warnings and semantic invariants in addition to byte equality.

#### Scenario: All missing artefacts are listed
- **WHEN** four part manifests lack render, symbol, and footprint goldens
- **THEN** the suite reports all twelve missing paths
- **AND** the failure is not limited to the first manifest

#### Scenario: Blessing is not enabled accidentally
- **WHEN** `COPPERLEAF_BLESS=0` is set
- **THEN** tests compare against existing goldens and do not write files

#### Scenario: Snapshot rewrite fails CI
- **WHEN** a test modifies a tracked golden during CI
- **THEN** the final worktree check fails

### Requirement: External KiCad acceptance is reproducible

Before a board-emission release claim, a pinned KiCad 10.x CLI acceptance suite SHALL load emitted `.kicad_pro`, `.kicad_sch`, and `.kicad_pcb` files, run ERC and DRC where applicable, check schematic parity, and compare results with a reviewed expected baseline. The suite SHALL use at least one real board and SHALL fail on unreviewed regressions.

#### Scenario: Invalid output fails to load
- **WHEN** KiCad rejects an emitted project file
- **THEN** acceptance fails and names the file and parser error

#### Scenario: New ERC or DRC finding fails acceptance
- **WHEN** an emitted board gains an electrical-rule or design-rule finding absent from the reviewed baseline
- **THEN** acceptance fails with the new finding

#### Scenario: Reviewed baseline is explicit
- **WHEN** a known limitation produces a documented ERC or DRC finding
- **THEN** acceptance records that finding in the baseline rather than reporting a blanket pass

### Requirement: Capability documentation matches implementation

README, crate documentation, CLI help, and OpenSpec SHALL state only capabilities that are implemented and tested. Unsupported layout operations SHALL be identified as unsupported, and an active change SHALL NOT retain checked tasks for absent functionality. Strict OpenSpec validation SHALL pass before a change is archived or a release is prepared.

#### Scenario: Unimplemented routing is not advertised
- **WHEN** the layout crate does not perform autorouting
- **THEN** its documentation and OpenSpec tasks describe placement-only behaviour
- **AND** routing is not marked complete

#### Scenario: Strict validation blocks archive
- **WHEN** a stable spec or active change has structural validation errors
- **THEN** strict validation fails
- **AND** archive preparation is blocked

### Requirement: Release is explicit and packaged output is verified

Workspace crates SHALL remain explicitly unpublished until a release decision. A publishable release SHALL require valid internal dependency versions, package metadata, third-party asset provenance, removal of developer-local paths, and successful build tests from each unpacked package.

#### Scenario: Unpublished workspace rejects publication
- **WHEN** a crate without publication approval is packaged for release
- **THEN** its manifest prevents publication

#### Scenario: Packaged crate builds in isolation
- **WHEN** a candidate release crate is unpacked outside the workspace
- **THEN** it builds using only declared registry and packaged dependencies
