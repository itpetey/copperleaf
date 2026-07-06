## Context

The current `Design` struct stores connectivity exclusively in `DesignGraph`, which is `#[serde(skip)]`. This means:

1. `serde_json::to_string(&design)` omits all wiring — the JSON has nets and components but no connections.
2. `serde_json::from_str::<Design>(json)` produces a design with an empty graph.
3. The `cl apply` command works only because it re-plays `Connect` patch ops after deserialization — it can't verify an already-connected design loaded from JSON.

The architecture doc (§7.1) calls for a "canonical, versioned schema ... designed for round-trip (load → modify → apply patch)." This is impossible today.

Additionally, `Qty<Second>` is used to represent signal bandwidth, but `50.0.mhz()` returns a period (1/50e6 seconds). The field is named `bandwidth`, so passing a frequency to get a period stored as "bandwidth" is inverted. Consumers like halow-sta hand-roll `1.0 / b.as_base() / 1.0e6` to display MHz.

## Goals / Non-Goals

**Goals:**
- Make `Design` serialize and deserialize with full connectivity (lossless round-trip).
- Keep `DesignGraph` as a derived index — the `connections` vec is the source of truth.
- Add `Qty<Second>::as_mhz()` for ergonomic frequency display.

**Non-Goals:**
- Changing the `DesignGraph` API surface (`pins_on_net`, `nets_of_pin`, `counts` stay the same).
- Adding a frequency unit type (`Qty<Hertz>`) — that's a larger refactor; `as_mhz()` is a stopgap.
- Changing how `connect()` is called — the signature stays `(refdes, pin, net)`.

## Decisions

### D1: `Connection` as a serializable record, not just graph edges

**Decision:** Add `Vec<Connection>` to `Design` as the source of truth for connectivity. The graph is rebuilt from it.

**Rationale:** The graph (`petgraph::Graph`) is not easily serializable and shouldn't be — it's an index. A flat `Vec<Connection>` is trivially serializable, human-readable in JSON, and can be replayed into a graph. This matches how the `apply` command already works (it replays connect ops).

**Alternative considered:** Serialize the graph's edge list directly. Rejected because `petgraph` edge indices are not stable across serializations, and the graph has extra `Node`/`Edge` enum types that add noise to JSON.

### D2: Custom Deserialize for Design

**Decision:** Implement a custom `Deserialize` that, after deserializing the struct fields, iterates `connections` and calls `graph.ensure_pin` + `graph.ensure_net` + `graph.add_edge` for each.

**Rationale:** Using `#[serde(deserialize_with)]` on the `graph` field alone is insufficient because graph building depends on `connections`, which is a sibling field. A small wrapper struct or a manual `Deserialize` impl is needed.

**Implementation approach:** Use a `#[derive(Deserialize)]` helper struct (`DesignRaw`) that mirrors `Design` minus the graph, then implement `Deserialize` for `Design` by deserializing `DesignRaw` and replaying connections into a fresh graph.

### D3: `connect()` writes to both `connections` and `graph`

**Decision:** `Design::connect()` pushes a `Connection` to `self.connections` and adds the edge to `self.graph`.

**Rationale:** Single write path, no sync ambiguity. The `connections` vec and graph are always consistent.

**Deduplication:** `connect()` already checks `graph.g.find_edge()` before adding — we should also check for duplicate `Connection` entries (same refdes+pin+net) to keep the vec clean.

### D4: `as_mhz()` on `Qty<Second>`

**Decision:** Add `pub fn as_mhz(&self) -> f64` to the `Qty<U>` impl block (or a specialized impl for `Qty<Second>`).

**Rationale:** `bandwidth` is stored as a period in seconds. `as_mhz()` returns `1.0 / self.as_base() / 1.0e6`. This is a one-liner that eliminates hand-rolled conversion in every consumer.

## Risks / Trade-offs

- **[Duplicate connections]** If `connect()` is called twice with the same args, the graph deduplication already prevents duplicate edges, but `connections` vec could grow. → Mitigate: check `connections` for existing entry before pushing.
- **[Deserialization cost]** Replaying N connections into a graph is O(N) — fine for designs with hundreds of connections. → Acceptable for v1.
- **[Breaking change]** `Design` gains a public field. Code using `Design { .. }` struct literals must add `connections: vec![]`. → Low impact: most code uses `Design::default()` or `Design::connect()`.
