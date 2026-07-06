## 1. Connection struct and Design field

- [ ] 1.1 Add `Connection` struct (`refdes: String, pin: String, net: String`) with `Clone, Debug, Serialize, Deserialize` derives to `crates/ir/src/lib.rs`
- [ ] 1.2 Add `connections: Vec<Connection>` field to `Design` struct (before the `#[serde(skip)] graph` field)
- [ ] 1.3 Update `Design::default()` (derive default) — `connections` defaults to `vec![]` automatically

## 2. Connect writes to both stores

- [ ] 2.1 Update `Design::connect()` to push a `Connection` to `self.connections` in addition to adding the graph edge
- [ ] 2.2 Add deduplication: skip pushing to `connections` if an identical `(refdes, pin, net)` entry already exists
- [ ] 2.3 Add a unit test verifying `connect()` populates both `connections` and the graph, and that duplicates are ignored

## 3. Lossless deserialization (graph rebuild)

- [ ] 3.1 Create a `DesignRaw` helper struct (private) that mirrors `Design` minus the `graph` field, deriving `Deserialize`
- [ ] 3.2 Implement `Deserialize` for `Design` that deserializes via `DesignRaw`, then replays `connections` into a fresh `DesignGraph`
- [ ] 3.3 Add a round-trip test: build a design with connections, serialize to JSON, deserialize, and assert `pins_on_net` and `nets_of_pin` match the original
- [ ] 3.4 Add a round-trip test asserting `graph.counts()` matches before and after serialization

## 4. as_mhz convenience method

- [ ] 4.1 Add `pub fn as_mhz(&self) -> f64` to the `impl<U: UnitMarker> Qty<U>` block in `crates/core/src/lib.rs` that returns `1.0 / self.as_base() / 1.0e6`
- [ ] 4.2 Add a unit test: `assert!((50.0.mhz().as_mhz() - 50.0).abs() < 1e-9)`

## 5. Update JSON snapshot test

- [ ] 5.1 Update `crates/ir/tests/json_snapshot.rs` expected JSON to include the `"connections"` array (empty for the no-connection test case, add a connection case if needed)
- [ ] 5.2 Add a second snapshot test case that includes at least one connection and verify the round-trip

## 6. Verify and validate

- [ ] 6.1 Run `cargo test -p copperleaf-ir` and ensure all tests pass
- [ ] 6.2 Run `cargo test -p copperleaf-core` and ensure all tests pass
- [ ] 6.3 Run `cargo build` across the workspace to verify no breaking compilation errors
- [ ] 6.4 Run `cargo test` across the workspace to ensure analysis and CLI tests still pass
