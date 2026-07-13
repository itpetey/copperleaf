## ADDED Requirements

### Requirement: Pin IDs are deterministic
`PinId` SHALL be a newtype wrapping a `String` produced by a deterministic hash function (FNV-1a 64-bit) seeded from stable inputs (component name and pin name). The same component and pin name SHALL always produce the same `PinId`. The `uuid` crate SHALL NOT be a dependency.

#### Scenario: Same pin name produces same ID
- **WHEN** two `Pin` instances are created with the same component name "U1" and pin name "VDD"
- **THEN** both have identical `PinId` values

#### Scenario: Different pin names produce different IDs
- **WHEN** two `Pin` instances are created with pin names "VDD" and "GND" on the same component
- **THEN** their `PinId` values differ

### Requirement: Deterministic UUIDs for backend emission
The backend SHALL use a deterministic UUID function (FNV-1a 64-bit hash formatted as 8-4-4-4-12 hex) for all UUID generation in KiCad output. The same `CompiledBoard` SHALL always produce identical output bytes, enabling reproducible builds, snapshot tests, and diff-based review.

#### Scenario: Same compiled board produces identical output
- **WHEN** the same `CompiledBoard` is emitted twice via the KiCad backend
- **THEN** both outputs are byte-for-byte identical

#### Scenario: Deterministic UUID is stable
- **WHEN** `deterministic_id("sch:U1")` is called twice
- **THEN** both calls return the same hex string

### Requirement: Random UUID generation is not used
No code in the workspace SHALL call `Uuid::new_v4()` or any other random ID generator. All IDs SHALL be derived from deterministic hashes of stable seeds. The `uuid` crate SHALL be removed from workspace dependencies.

#### Scenario: No uuid dependency
- **WHEN** the workspace `Cargo.toml` is inspected
- **THEN** `uuid` is not listed in `[workspace.dependencies]`
