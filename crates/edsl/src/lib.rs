//! Embedded DSL (EDSL) for building Copperleaf designs.
//!
//! This crate provides lightweight macros to construct designs succinctly in
//! examples and tests. Macros are deliberately limited and subject to change.

pub use copperleaf_ir::*;

// Macro stubs retained for API exploration
/// Create a [`copperleaf_ir::Design`] with a convenient builder-like block.
///
/// Example:
/// `let d = design!("demo", |design| { design.add_net(Net::ground()); });`
#[macro_export]
macro_rules! design {
    ($name:literal, |$d:ident| $body:block) => {{
        let mut $d = ::copperleaf_ir::Design::default();
        $body
        $d
    }};
}

#[macro_export]
/// Connect a single pin to a net or apply a list of connections.
macro_rules! connect {
    // Single connection: connect!(design, "U1", "VDD", "V3V3");
    ($d:expr, $refdes:expr, $pin:expr, $net:expr) => {{
        $d.connect($refdes, $pin, $net);
    }};
    // List of connections: connect!(design, [("U1","VDD","V3V3"), ("U2","VSS","GND")]);
    ($d:expr, [ $( ($refdes:expr, $pin:expr, $net:expr) ),+ $(,)? ]) => {{
        $( $d.connect($refdes, $pin, $net); )+
    }};
}

#[macro_export]
/// Placeholder macro for future inline verification in the EDSL.
macro_rules! verify {
    ($($tt:tt)*) => {
        compile_error!("verify! macro is a stub in edsl")
    };
}

#[macro_export]
/// Placeholder macro for future export helpers in the EDSL.
macro_rules! export {
    ($($tt:tt)*) => {
        compile_error!("export! macro is a stub in edsl")
    };
}
