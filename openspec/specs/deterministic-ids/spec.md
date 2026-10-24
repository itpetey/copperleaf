# deterministic-ids Specification

## Purpose
Guarantee that pin and emitted-object identifiers are stable, collision-free, and reproducible across runs, so generated output can be snapshotted and reviewed byte-for-byte.
## Requirements
### Requirement: Pin IDs are deterministic
`PinId` SHALL be a newtype wrapping a `String` produced by a deterministic hash function (FNV-1a 64-bit) seeded from the component refdes and its canonical physical-pin identity. The same physical pin SHALL always produce the same `PinId`. Pins sharing a display name but having different pad numbers SHALL produce different IDs. The `uuid` crate SHALL NOT be a dependency.

#### Scenario: Same physical pin produces same ID
- **WHEN** two `Pin` instances are created for component `U1` with pad number `7` and pin name `VDD`
- **THEN** both have identical `PinId` values

#### Scenario: Repeated names produce different IDs
- **WHEN** component `U1` has pins named `AVDD` with pad numbers `4` and `8`
- **THEN** their `PinId` values differ

#### Scenario: Same pad number on different components differs
- **WHEN** `U1` and `U2` each have pad number `7`
- **THEN** their `PinId` values differ

### Requirement: Deterministic UUIDs for backend emission
The backend SHALL use a deterministic UUID function (FNV-1a 64-bit hash formatted as 8-4-4-4-12 hex) for all UUID generation in KiCad output. Seeds SHALL include the addressable object class and every input needed to distinguish siblings, including refdes or net, pad number, segment end or ordinal, via end layer or ordinal, zone ordinal, and property owner. Every UUID within one emitted project SHALL be unique while remaining byte-reproducible.

#### Scenario: Same compiled board produces identical output
- **WHEN** the same `CompiledBoard` is emitted twice via the KiCad backend
- **THEN** both outputs are byte-for-byte identical

#### Scenario: Same-named properties have unique UUIDs
- **WHEN** multiple capacitor footprints emit the same hidden `Value` property
- **THEN** each property UUID differs
- **AND** every UUID in the emitted project is unique

#### Scenario: Sibling copper objects have unique UUIDs
- **WHEN** two segments share a net, layer, and start point
- **THEN** their UUIDs differ because their end points or ordinals differ

#### Scenario: Deterministic UUID is stable
- **WHEN** `deterministic_id("sch:U1")` is called twice
- **THEN** both calls return the same hex string

### Requirement: Random UUID generation is not used
No code in the workspace SHALL call `Uuid::new_v4()` or any other random ID generator. All IDs SHALL be derived from deterministic hashes of stable seeds. The `uuid` crate SHALL be removed from workspace dependencies.

#### Scenario: No uuid dependency
- **WHEN** the workspace `Cargo.toml` is inspected
- **THEN** `uuid` is not listed in `[workspace.dependencies]`

