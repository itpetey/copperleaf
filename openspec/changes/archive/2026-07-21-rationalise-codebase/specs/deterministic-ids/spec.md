## MODIFIED Requirements

### Requirement: Deterministic UUIDs for backend emission
The backend SHALL use the core deterministic ID function (FNV-1a 64-bit hash formatted as 8-4-4-4-12 hex) for all UUID generation in KiCad output; the backend SHALL NOT carry its own copy of the hash/format logic. The same `CompiledBoard` SHALL always produce identical output bytes — including across separate operating-system processes — enabling reproducible builds, snapshot tests, and diff-based review.

#### Scenario: Same compiled board produces identical output in one process
- **WHEN** the same `CompiledBoard` is emitted twice via the KiCad backend
- **THEN** both outputs are byte-for-byte identical

#### Scenario: Same compiled board produces identical output across processes
- **WHEN** the same board is compiled and emitted in two separate processes
- **THEN** all emitted files are byte-for-byte identical

#### Scenario: Deterministic UUID is stable
- **WHEN** `deterministic_id("sch:U1")` is called twice
- **THEN** both calls return the same hex string
