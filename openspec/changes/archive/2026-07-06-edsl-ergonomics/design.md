## Context

The halow-sta project's `reference_design.rs` has ~150 lines of passive wiring boilerplate (lines 223-351). Each passive follows the same 5-line pattern:

```rust
let c = Capacitor::new("C10", 4.7.uf());
let c10_inst = ComponentInst::new("C10", c);
d.add_component(&c10_inst);
d.connect("C10", "1", "W5500_TOCAP");
d.connect("C10", "2", "GND");
```

IC-to-IC connections require two calls each:
```rust
d.connect("U1", "SDIO_CLK", "SDIO_CLK");
d.connect("U2", "GPIO2", "SDIO_CLK");
```

Part definitions (ht_hc01.rs: 190 lines, rp2354a.rs: 123, w5500.rs: 181) are all struct + `new()` + `impl Block` with hand-built `Vec<Pin>`. The `Block` trait's `id()` method is redundant — `ComponentInst` already carries `refdes`, and `ComponentRecord` carries `refdes`. The `id` field on each part struct is stored, returned by `id()`, and never used for anything else.

`add_component()` takes `&ComponentInst<B>` by reference, which is awkward — the original `ComponentInst` is typically dropped after the call.

## Goals / Non-Goals

**Goals:**
- Reduce passive wiring from 5 lines to 1 (`add_cap`).
- Reduce IC-to-IC wiring from 2 calls to 1 (`connect_net`).
- Reduce part definitions from 150+ lines to ~40 (`part!` macro).
- Simplify `Block` trait by removing redundant `id()`.

**Non-Goals:**
- Bus templates (e.g., `spi_bus()`) — complex enough to be a separate change.
- A full derive macro (`#[derive(Component)]`) — `part!` is simpler and doesn't require a proc-macro crate.
- Removing `ComponentInst` entirely — it's still useful as a wrapper; just changing `add_component` to consume it.

## Decisions

### D1: `wire()` parses `"refdes.pin"` notation

**Decision:** `Design::wire(&mut self, pin: &str, net: &str)` parses `pin` as `"refdes.pin"`, splits on `.`, and calls `connect()`.

**Rationale:** Eliminates the 3-arg `connect("U1", "VDD", "V3V3")` pattern in favor of `wire("U1.VDD", "V3V3")`. More readable, especially when listing many connections.

**Error handling:** If the string has no `.`, panic with a clear message. This is a builder-time API, not a runtime API.

### D2: `connect_net()` connects multiple pins to one net

**Decision:** `Design::connect_net(&mut self, net: &str, pins: &[&str])` iterates `pins` and calls `wire(pin, net)` for each.

**Rationale:** An SPI clock net connects 2+ pins. Writing `d.connect_net("SDIO_CLK", &["U1.SDIO_CLK", "U2.GPIO2"])` is one line instead of two `connect()` calls.

### D3: `add_cap()` / `add_res()` one-liners

**Decision:** `Design::add_cap(&mut self, refdes: &str, value: Qty<Farad>, net_pos: &str, net_neg: &str)` constructs a `Capacitor`, wraps in `ComponentInst`, adds to design, and wires both pins. Same for `add_res()`.

**Rationale:** Collapses 5 lines to 1. Depends on `Capacitor`/`Resistor` from `analysis-stdlib`. These methods are on `Design` (in `copperleaf-ir`), but they need the passive types. To avoid a circular dependency (`ir` → `parts` → `ir`), the methods are generic: `add_cap<C: Block>(...)` takes a factory closure, or the methods live in `copperleaf-edsl` which depends on both `ir` and `parts`.

**Implementation choice:** Put `add_cap`/`add_res` in `copperleaf-edsl` as extension methods on `Design` via a trait. This keeps `ir` dependency-free on `parts`.

### D4: `part!` macro

**Decision:** A declarative macro in `copperleaf-edsl` that generates a part struct, `new()`, and `impl Block` from a pin list and constraint list.

**Syntax:**
```rust
part! {
    pub struct HtHc01("HT-HC01_V2");
    pins:
        GND = gnd(),
        SDIO_D0 = dio_spi(50.0.mhz(), 50.0.ohm()),
        VDD_IO = power_in(1.62.volt(), 3.6.volt(), 0.2.amp()),
        NC = dio(),
        ;
    constraints:
        Decoupling { values: [100.0.nf(), 10.0.uf()], per_pin: true },
        LengthMatch { group: "SPI_BUS", skew_ps: 200.0 },
        ;
}
```

**Rationale:** Declarative macros don't need a proc-macro crate. The pin helpers (`gnd()`, `dio()`, `power_in()`, etc.) are small functions that return `Pin`. The macro generates the `Vec<Pin>` and the `impl Block`.

### D5: Remove `id()` from `Block`

**Decision:** Remove `fn id(&self) -> &str` from the `Block` trait. Parts no longer need to store an `id: String`.

**Rationale:** `id()` duplicates the refdes on `ComponentInst`/`ComponentRecord`. It's never used by analysis passes or the graph. Removing it simplifies the trait and eliminates the `id: String` field from every part struct.

**Breaking impact:** Every `impl Block` must remove the `id()` method. Every part struct must remove the `id` field and `id: id.to_owned()` in `new()`.

### D6: `add_component` consumes `ComponentInst`

**Decision:** `fn add_component<B: Block>(&mut self, inst: ComponentInst<B>)` (by value, not by reference).

**Rationale:** The reference pattern `add_component(&inst)` is awkward — the `ComponentInst` is typically created inline and not used again. Consuming it is more ergonomic: `d.add_component(ComponentInst::new("U1", block))`.

## Risks / Trade-offs

- **[Breaking changes]** Removing `id()` and changing `add_component` signature break all existing `Block` impls and all `add_component` call sites. → Acceptable: copperleaf is pre-1.0, breaking changes are expected, and the migration is mechanical.
- **[Macro complexity]** Declarative macros have limitations (no expression interpolation for pin helper args). → The `part!` macro calls existing functions; it doesn't need to parse arbitrary expressions, just function call patterns.
- **[Extension trait for add_cap/add_res]** Putting them in `edsl` means users must `use copperleaf::edsl::*` to get them. → Acceptable: they're already re-exported through the facade.
