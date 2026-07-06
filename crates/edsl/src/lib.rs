//! Embedded DSL (EDSL) for building Copperleaf designs.
//!
//! This crate provides lightweight macros to construct designs succinctly in
//! examples and tests. Macros are deliberately limited and subject to change.

pub use copperleaf_ir::*;
use copperleaf_core::{Amp, Ohm, Qty, Second, UnitExt, Volt};

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

// Pin helper functions used by `part!` and available for direct use.

/// Ground pin helper.
pub fn gnd() -> Pin {
    Pin::new(
        "GND",
        Role::Gnd,
        Limits::new(0.0.volt(), 0.0.volt(), 100.0.amp()),
        None,
    )
}

/// Generic digital I/O pin helper.
pub fn dio() -> Pin {
    Pin::new(
        "DIO",
        Role::DigitalIO,
        Limits::new(0.0.volt(), 3.6.volt(), 0.1.amp()),
        None,
    )
}

/// Power input pin helper with the given voltage/current limits.
pub fn power_in(v_min: Qty<Volt>, v_max: Qty<Volt>, i_max: Qty<Amp>) -> Pin {
    Pin::new("PWR", Role::PowerIn, Limits::new(v_min, v_max, i_max), None)
}

/// SPI data pin helper with bandwidth (as period) and target impedance.
pub fn dio_spi(bw: Qty<Second>, z: Qty<Ohm>) -> Pin {
    Pin::new(
        "SPI",
        Role::DigitalIO,
        Limits::new(0.0.volt(), 3.6.volt(), 0.1.amp()),
        Some(SigSpec {
            kind: SigKind::Generic,
            bandwidth: Some(bw),
            edge_rate: None,
            target_impedance: Some(z),
        }),
    )
}

/// SPI clock pin helper with bandwidth (as period) and target impedance.
pub fn dio_clk(bw: Qty<Second>, z: Qty<Ohm>) -> Pin {
    Pin::new(
        "CLK",
        Role::DigitalIO,
        Limits::new(0.0.volt(), 3.6.volt(), 0.1.amp()),
        Some(SigSpec {
            kind: SigKind::Clock,
            bandwidth: Some(bw),
            edge_rate: None,
            target_impedance: Some(z),
        }),
    )
}

/// Analog input pin helper with the given limits.
pub fn analog_in(limits: Limits) -> Pin {
    Pin::new("AIN", Role::AnalogIn, limits, None)
}

/// Declarative macro for defining parts from a pin table.
///
/// Syntax:
/// ```ignore
/// part! {
///     pub struct MyChip("MYCHIP");
///     pins:
///         VDD = power_in(1.7.volt(), 3.6.volt(), 0.5.amp()),
///         GND = gnd(),
///         ;
///     constraints:
///         Decoupling { values: [100.0.nf()], per_pin: true },
///         ;
/// }
/// ```
#[macro_export]
macro_rules! part {
    (
        $vis:vis struct $name:ident($default_id:literal);
        pins:
            $( $pin_name:ident = $pin_expr:expr ),*
            $(,)?
            ;
        $(
            constraints:
                $( $constraint:expr ),*
                $(,)?
                ;
        )?
    ) => {
        #[derive(Clone, Debug)]
        $vis struct $name {
            pins: ::std::vec::Vec<$crate::Pin>,
        }

        impl $name {
            /// Create a new part instance. The `id` argument is accepted for
            /// API compatibility but is not stored (identity lives on
            /// [`ComponentInst`]).
            pub fn new(_id: &str) -> Self {
                Self {
                    pins: vec![
                        $(
                            $crate::Pin::duplicate(&$pin_expr, stringify!($pin_name))
                        ),*
                    ],
                }
            }
        }

        impl $crate::Block for $name {
            fn pins(&self) -> &[$crate::Pin] {
                &self.pins
            }

            fn constraints(&self) -> ::std::vec::Vec<$crate::Constraint> {
                vec![
                    $( $( $constraint, )* )?
                ]
            }
        }
    };
}

#[cfg(feature = "parts")]
pub mod design_ext {
    //! Extension methods on [`Design`] for adding passive components in one call.

    use copperleaf_parts::{Capacitor, Resistor};
    use copperleaf_ir::{ComponentInst, Design};
    use copperleaf_core::{Farad, Ohm, Qty};

    /// Extension trait adding passive convenience methods to [`Design`].
    pub trait DesignExt {
        /// Add a capacitor with the given value and wire both pins.
        fn add_cap(&mut self, refdes: &str, value: Qty<Farad>, net_pos: &str, net_neg: &str);

        /// Add a resistor with the given value and wire both pins.
        fn add_res(&mut self, refdes: &str, value: Qty<Ohm>, net_a: &str, net_b: &str);
    }

    impl DesignExt for Design {
        fn add_cap(&mut self, refdes: &str, value: Qty<Farad>, net_pos: &str, net_neg: &str) {
            let c = Capacitor::new(value);
            let inst = ComponentInst::new(refdes, c);
            self.add_component(inst);
            self.wire(&format!("{}.{}", refdes, "1"), net_pos);
            self.wire(&format!("{}.{}", refdes, "2"), net_neg);
        }

        fn add_res(&mut self, refdes: &str, value: Qty<Ohm>, net_a: &str, net_b: &str) {
            let r = Resistor::new(value);
            let inst = ComponentInst::new(refdes, r);
            self.add_component(inst);
            self.wire(&format!("{}.{}", refdes, "1"), net_a);
            self.wire(&format!("{}.{}", refdes, "2"), net_b);
        }
    }
}

#[cfg(feature = "parts")]
pub use design_ext::DesignExt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gnd_helper_returns_gnd_pin() {
        let p = gnd();
        assert_eq!(p.name, "GND");
        assert!(matches!(p.role, Role::Gnd));
    }

    #[test]
    fn dio_helper_returns_digital_io_pin() {
        let p = dio();
        assert_eq!(p.name, "DIO");
        assert!(matches!(p.role, Role::DigitalIO));
        assert!(p.sig.is_none());
    }

    #[test]
    fn power_in_helper_returns_power_in_pin() {
        let p = power_in(1.62.volt(), 3.6.volt(), 0.2.amp());
        assert_eq!(p.name, "PWR");
        assert!(matches!(p.role, Role::PowerIn));
        assert!((p.limits.v_min.as_base() - 1.62).abs() < 1e-9);
        assert!((p.limits.v_max.as_base() - 3.6).abs() < 1e-9);
        assert!((p.limits.i_max.as_base() - 0.2).abs() < 1e-9);
    }

    #[test]
    fn dio_spi_helper_returns_spi_sig_spec() {
        let p = dio_spi(50.0.mhz(), 50.0.ohm());
        assert_eq!(p.name, "SPI");
        assert!(matches!(p.role, Role::DigitalIO));
        let sig = p.sig.expect("sig spec present");
        assert!(matches!(sig.kind, SigKind::Generic));
        assert!((sig.bandwidth.unwrap().as_mhz() - 50.0).abs() < 1e-9);
        assert!((sig.target_impedance.unwrap().as_base() - 50.0).abs() < 1e-9);
    }

    #[test]
    fn dio_clk_helper_returns_clock_sig_spec() {
        let p = dio_clk(50.0.mhz(), 50.0.ohm());
        assert_eq!(p.name, "CLK");
        assert!(matches!(p.role, Role::DigitalIO));
        let sig = p.sig.expect("sig spec present");
        assert!(matches!(sig.kind, SigKind::Clock));
    }

    #[test]
    fn analog_in_helper_returns_analog_in_pin() {
        let limits = Limits::new(0.0.volt(), 3.3.volt(), 0.01.amp());
        let p = analog_in(limits);
        assert_eq!(p.name, "AIN");
        assert!(matches!(p.role, Role::AnalogIn));
        assert!((p.limits.v_max.as_base() - limits.v_max.as_base()).abs() < 1e-9);
    }

    #[test]
    fn part_macro_generates_struct_with_pins() {
        part! {
            pub struct MyChip("MYCHIP");
            pins:
                VDD = power_in(1.7.volt(), 3.6.volt(), 0.5.amp()),
                GND = gnd(),
                ;
        }

        let chip = MyChip::new("U1");
        assert_eq!(chip.pins().len(), 2);
        assert_eq!(chip.pins()[0].name, "VDD");
        assert!(matches!(chip.pins()[0].role, Role::PowerIn));
        assert!((chip.pins()[0].limits.v_min.as_base() - 1.7).abs() < 1e-9);
        assert!((chip.pins()[0].limits.v_max.as_base() - 3.6).abs() < 1e-9);
        assert!((chip.pins()[0].limits.i_max.as_base() - 0.5).abs() < 1e-9);
        assert_eq!(chip.pins()[1].name, "GND");
        assert!(matches!(chip.pins()[1].role, Role::Gnd));
    }

    #[test]
    fn part_macro_supports_duplicate_pin_names() {
        part! {
            struct MultiGnd("MULTIGND");
            pins:
                GND = gnd(),
                GND = gnd(),
                GND = gnd(),
                ;
        }

        let chip = MultiGnd::new("U1");
        assert_eq!(chip.pins().len(), 3);
        assert!(chip.pins().iter().all(|p| p.name == "GND" && matches!(p.role, Role::Gnd)));
    }

    #[test]
    fn part_macro_supports_constraints_section() {
        part! {
            struct ConstrainedChip("CHIP");
            pins:
                VDD = power_in(1.7.volt(), 3.6.volt(), 0.5.amp()),
                ;
            constraints:
                Constraint::Decoupling { values: vec![100.0.nf()], per_pin: true },
                Constraint::MaxJunction { temp: 125.0.celsius() },
                ;
        }

        let chip = ConstrainedChip::new("U1");
        assert_eq!(chip.constraints().len(), 2);
        assert!(matches!(chip.constraints()[0], Constraint::Decoupling { .. }));
        assert!(matches!(chip.constraints()[1], Constraint::MaxJunction { .. }));
    }

    #[test]
    #[cfg(feature = "parts")]
    fn add_cap_adds_component_and_wires_pins() {
        use crate::DesignExt;

        let mut d = Design::default();
        d.add_net(Net::power("VDD", 3.3.volt()));
        d.add_net(Net::ground());
        d.add_cap("C1", 100.0.nf(), "VDD", "GND");

        assert!(d.component_by_refdes("C1").is_some());
        assert!(d.pins_on_net("VDD").contains(&("C1".into(), "1".into())));
        assert!(d.pins_on_net("GND").contains(&("C1".into(), "2".into())));
    }

    #[test]
    #[cfg(feature = "parts")]
    fn add_res_adds_component_and_wires_pins() {
        use crate::DesignExt;

        let mut d = Design::default();
        d.add_net(Net::power("VDD", 3.3.volt()));
        d.add_net(Net::power("SDIO_CS", 3.3.volt()));
        d.add_res("R1", 10.0.kohm(), "VDD", "SDIO_CS");

        assert!(d.component_by_refdes("R1").is_some());
        assert!(d.pins_on_net("VDD").contains(&("R1".into(), "1".into())));
        assert!(d.pins_on_net("SDIO_CS").contains(&("R1".into(), "2".into())));
    }
}
