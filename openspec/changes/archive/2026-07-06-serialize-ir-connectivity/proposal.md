## Why

The JSON IR is lossy: `DesignGraph` is `#[serde(skip)]`, so all connectivity is lost on serialization. A design serialized to JSON and deserialized back has an empty graph with zero connections. This blocks the CLI from loading external designs, prevents AI round-trip workflows (the patch protocol in ARCHITECTURE.md §7.2 assumes load-modify-save), and makes the JSON IR fundamentally incomplete. Additionally, `mhz()` returns a `Qty<Second>` (period) stored in a field called `bandwidth`, which inverts semantics and forces consumers to hand-roll `1.0 / b.as_base() / 1.0e6` to display MHz.

## What Changes

- Add a `Connection` struct (`refdes: String, pin: String, net: String`) to `copperleaf-ir`.
- Add `connections: Vec<Connection>` to `Design`, serialized as part of the JSON IR.
- `Design::connect()` now pushes to both `connections` and the graph, keeping them in sync.
- Implement `Deserialize` for `Design` that replays `connections` into a fresh `DesignGraph`, making round-trips lossless.
- Add `Qty<Second>::as_mhz()` convenience method so consumers don't hand-roll frequency conversion.
- Update the JSON snapshot test to include connections in the expected output.
- **BREAKING**: `Design` gains a public `connections` field. Any code that constructs `Design` with `..Default::default()` will pick it up automatically, but direct struct construction must account for the new field.

## Capabilities

### New Capabilities
- `ir-connectivity`: Serializable connection records in the design IR, enabling lossless JSON round-trips and graph rebuilding on deserialization.

### Modified Capabilities

## Impact

- **`crates/ir/src/lib.rs`**: New `Connection` struct, modified `Design` struct, updated `connect()` impl, custom `Deserialize` impl or `#[serde(deserialize_with = ...)]` for graph rebuilding.
- **`crates/ir/tests/json_snapshot.rs`**: Expected JSON must include `connections` array.
- **`crates/core/src/lib.rs`**: Add `as_mhz()` to `Qty<Second>` impl block.
- **`crates/analysis/src/lib.rs`**: No changes needed — analysis already works on `&Design` and calls `connect()`/`nets_of_pin()` which will use the new backing store transparently.
- **`crates/cli/src/main.rs`**: The `apply` command already replays connections; it will now work correctly on deserialized designs because the graph will be rebuilt.
