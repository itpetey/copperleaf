## 1. Remove id() from Block trait

- [x] 1.1 Remove `fn id(&self) -> &str` from the `Block` trait in `crates/ir/src/lib.rs`
- [x] 1.2 Remove `id` field from `Buck` and `Mcu` structs in `crates/parts/src/lib.rs`, update `new()` and `impl Block`
- [x] 1.3 Update `crates/copperleaf/examples/basic.rs` and `hello.rs` to remove any `id()` calls
- [x] 1.4 Run `cargo build` to verify compilation

## 2. add_component consumes ComponentInst

- [x] 2.1 Change `Design::add_component` signature from `&ComponentInst<B>` to `ComponentInst<B>` (by value) in `crates/ir/src/lib.rs`
- [x] 2.2 Update all call sites: `crates/analysis/src/lib.rs` tests, `crates/cli/src/main.rs`, `crates/copperleaf/examples/`
- [x] 2.3 Run `cargo build` and `cargo test` to verify

## 3. Connection helper methods on Design

- [x] 3.1 Add `Design::wire(&mut self, pin: &str, net: &str)` that splits `"refdes.pin"` on `.` and calls `connect()`
- [x] 3.2 Add `Design::connect_net(&mut self, net: &str, pins: &[&str])` that calls `wire()` for each pin
- [x] 3.3 Add unit tests for `wire()` and `connect_net()`

## 4. Passive convenience methods (in edsl crate)

- [x] 4.1 Add an extension trait `DesignExt` in `crates/edsl/src/lib.rs` with `add_cap()`, `add_res()` methods
- [x] 4.2 `add_cap(refdes, value, net_pos, net_neg)` constructs `Capacitor::new()`, wraps in `ComponentInst`, calls `add_component()`, wires both pins
- [x] 4.3 `add_res(refdes, value, net_a, net_b)` same pattern with `Resistor::new()`
- [x] 4.4 Add unit tests for `add_cap()` and `add_res()` verifying component exists and pins are wired
- [x] 4.5 Re-export the extension trait through the `copperleaf` facade

## 5. Pin helper functions

- [x] 5.1 Add pin helper functions to `crates/edsl/src/lib.rs`: `gnd()`, `dio()`, `power_in(v_min, v_max, i_max)`, `dio_spi(bw, z)`, `dio_clk(bw, z)`, `analog_in(limits)`
- [x] 5.2 Each returns a `Pin` with appropriate role, limits, and sig spec
- [x] 5.3 Add unit tests verifying pin helper output

## 6. part! macro

- [x] 6.1 Design the `part!` macro syntax: `part! { pub struct Name("default_id"); pins: ... ; constraints: ... ; }`
- [x] 6.2 Implement the macro in `crates/edsl/src/lib.rs` generating struct, `new()`, and `impl Block`
- [x] 6.3 Support pin helper function calls in the pin table
- [x] 6.4 Support duplicate pin names (multiple GND, etc.)
- [x] 6.5 Support optional constraints section
- [x] 6.6 Add a test that defines a small part via `part!` and verifies pin count, pin roles, and constraints

## 7. Validate

- [x] 7.1 Run `cargo test` across the workspace
- [x] 7.2 Run `cargo clippy --all-targets -- -D warnings` if available
- [x] 7.3 Verify that the `edsl` crate compiles without `parts` as a direct dependency (passive types are passed via generics or the extension trait is gated)
