## Why

Building a design in copperleaf requires extreme boilerplate. A single decoupling capacitor takes 5 lines (construct block, wrap in ComponentInst, add to design, connect pin 1, connect pin 2). With 30 passives, that's 150 lines of ceremony. Every IC-to-IC connection requires two separate `connect()` calls. Part definitions are 100-190 lines of hand-built `Vec<Pin>` with no declarative shorthand. The `Block` trait carries a redundant `id()` method that duplicates the refdes already stored on `ComponentInst`. These pain points make copperleaf designs 3-4x longer than necessary.

## What Changes

- Add `Design::wire(refdes.pin, net)` shorthand that parses `"U1.VDD"` and calls `connect()`.
- Add `Design::connect_net(net, &[pins])` that connects multiple pins to one net in one call.
- Add `Design::add_cap(refdes, value, net_pos, net_neg)` and `Design::add_res(refdes, value, net_a, net_b)` convenience methods that construct, add, and wire a passive in one call. Depends on `analysis-stdlib` for passive types.
- Add a `part!` declarative macro to `copperleaf-edsl` that generates the struct, `new()`, and `impl Block` from a pin table.
- **BREAKING**: Remove `id()` from the `Block` trait (it duplicates refdes and is never used for anything meaningful). Parts no longer store an `id: String` field.
- **BREAKING**: `Design::add_component()` consumes `ComponentInst` instead of taking it by reference.

## Capabilities

### New Capabilities
- `connection-helpers`: Shorthand methods on `Design` for wiring pins and adding passives in a single call.
- `part-macro`: Declarative macro for defining parts from a pin table, eliminating hand-built `Vec<Pin>` boilerplate.

### Modified Capabilities

## Impact

- **`crates/ir/src/lib.rs`**: Add `wire()`, `connect_net()`, `add_cap()`, `add_res()` to `Design`. Modify `add_component()` to consume. Remove `id()` from `Block` trait.
- **`crates/edsl/src/lib.rs`**: Add `part!` macro.
- **`crates/parts/src/lib.rs`**: Update `Buck` and `Mcu` to remove `id` field and `id()` method.
- **`crates/copperleaf/examples/`**: Update examples for new `add_component()` signature.
- **All consumer projects**: Must update `Block` impls (remove `id()`), change `add_component(&inst)` to `add_component(inst)`, and can optionally adopt `wire()`/`add_cap()`/`part!`.
