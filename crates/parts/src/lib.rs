//! Library of common parts used in examples and tests.
//!
//! These part models intentionally focus on pins and essential constraints only.

use copperleaf_core::{Amp, Farad, Henry, Ohm, Qty, Second, UnitExt, Volt};
use copperleaf_ir::*;

/// Simple synchronous buck regulator model with minimal pins and ratings.
#[derive(Clone, Debug)]
pub struct Buck {
    pins: Vec<Pin>,
    pub v_out: Qty<Volt>,
    pub i_max: Qty<Amp>,
}

/// Generic microcontroller model with USB pins and power rails.
#[derive(Clone, Debug)]
pub struct Mcu {
    pins: Vec<Pin>,
}

/// Standard two-pin capacitor.
#[derive(Clone, Debug)]
pub struct Capacitor {
    pins: Vec<Pin>,
    pub value: Qty<Farad>,
}

/// Standard two-pin resistor.
#[derive(Clone, Debug)]
pub struct Resistor {
    pins: Vec<Pin>,
    pub value: Qty<Ohm>,
    /// Net the resistor terminates on (e.g., `VCC` for a pull-up, `GND` for a pull-down).
    pub net: String,
}

/// Standard two-pin crystal.
#[derive(Clone, Debug)]
pub struct Crystal {
    pins: Vec<Pin>,
    pub frequency: Qty<Second>,
}

/// Standard two-pin inductor.
#[derive(Clone, Debug)]
pub struct Inductor {
    pins: Vec<Pin>,
    pub value: Qty<Henry>,
}

impl Buck {
    /// Create a new buck regulator with output voltage and current limit.
    pub fn new(v_out: Qty<Volt>, i_max: Qty<Amp>) -> Self {
        Self {
            v_out,
            i_max,
            pins: vec![
                Pin {
                    name: "VIN".into(),
                    role: Role::PowerIn,
                    limits: Limits {
                        v_min: 3.0.volt(),
                        v_max: 24.0.volt(),
                        i_max: 3.0.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
                Pin {
                    name: "SW".into(),
                    role: Role::PowerOut,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 24.0.volt(),
                        i_max: 3.0.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
                Pin {
                    name: "GND".into(),
                    role: Role::Gnd,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 0.0.volt(),
                        i_max: 100.0.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
            ],
        }
    }
}

impl Block for Buck {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
    fn constraints(&self) -> Vec<Constraint> {
        vec![Constraint::Decoupling {
            values: vec![100.0.nf(), 1.0.uf()],
            per_pin: true,
        }]
    }
}

impl Mcu {
    /// Create a new MCU with a small selection of common pins.
    pub fn new() -> Self {
        let usb_spec = SigSpec {
            kind: SigKind::Usb2Hs,
            bandwidth: Some(480.0.mhz()),
            edge_rate: None,
            target_impedance: Some(90.0.ohm()),
        };
        Self {
            pins: vec![
                Pin {
                    name: "VDD".into(),
                    role: Role::PowerIn,
                    limits: Limits {
                        v_min: 1.7.volt(),
                        v_max: 3.6.volt(),
                        i_max: 0.5.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
                Pin {
                    name: "VSS".into(),
                    role: Role::Gnd,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 0.0.volt(),
                        i_max: 100.0.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
                Pin {
                    name: "USB_DP".into(),
                    role: Role::DiffPos,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        i_max: 0.05.amp(),
                    },
                    sig: Some(usb_spec),
                    pos: None,
                    rotation: None,
                    length: None,
                },
                Pin {
                    name: "USB_DM".into(),
                    role: Role::DiffNeg,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        i_max: 0.05.amp(),
                    },
                    sig: Some(usb_spec),
                    pos: None,
                    rotation: None,
                    length: None,
                },
            ],
        }
    }
}

impl Block for Mcu {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
    fn constraints(&self) -> Vec<Constraint> {
        vec![
            Constraint::Impedance {
                target: 90.0.ohm(),
                tol_pct: 10.0,
            },
            Constraint::LengthMatch {
                group: "USB_D".into(),
                skew_ps: 200.0,
            },
        ]
    }
}

impl Default for Mcu {
    fn default() -> Self {
        Self::new()
    }
}

impl Capacitor {
    /// Create a generic two-pin capacitor with the given reference designator and value.
    pub fn new(value: Qty<Farad>) -> Self {
        Self {
            value,
            pins: vec![
                Pin {
                    name: "1".into(),
                    role: Role::DigitalIO,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        i_max: 0.1.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
                Pin {
                    name: "2".into(),
                    role: Role::DigitalIO,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        i_max: 0.1.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
            ],
        }
    }

    /// Create a decoupling capacitor with PowerIn and Gnd pins rated for 50 V.
    pub fn decoupling(value: Qty<Farad>) -> Self {
        Self {
            value,
            pins: vec![
                Pin {
                    name: "1".into(),
                    role: Role::PowerIn,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 50.0.volt(),
                        i_max: 0.1.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
                Pin {
                    name: "2".into(),
                    role: Role::Gnd,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 0.0.volt(),
                        i_max: 0.1.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
            ],
        }
    }
}

impl Block for Capacitor {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

impl Resistor {
    /// Create a generic two-pin resistor.
    pub fn new(value: Qty<Ohm>) -> Self {
        Self {
            value,
            net: String::new(),
            pins: vec![
                Pin {
                    name: "1".into(),
                    role: Role::DigitalIO,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        i_max: 0.1.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
                Pin {
                    name: "2".into(),
                    role: Role::DigitalIO,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        i_max: 0.1.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
            ],
        }
    }

    /// Create a pull-up resistor connected to `net`.
    pub fn pullup(value: Qty<Ohm>, net: &str) -> Self {
        Self {
            value,
            net: net.to_owned(),
            pins: vec![
                Pin {
                    name: "1".into(),
                    role: Role::DigitalIO,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        i_max: 0.1.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
                Pin {
                    name: "2".into(),
                    role: Role::DigitalIO,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        i_max: 0.1.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
            ],
        }
    }

    /// Create a pull-down resistor connected to `net`.
    pub fn pulldown(value: Qty<Ohm>, net: &str) -> Self {
        Self::pullup(value, net)
    }
}

impl Block for Resistor {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

impl Crystal {
    /// Create a two-pin crystal with both pins as AnalogIn inputs.
    pub fn new(frequency: Qty<Second>) -> Self {
        Self {
            frequency,
            pins: vec![
                Pin {
                    name: "1".into(),
                    role: Role::AnalogIn,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        i_max: 0.01.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
                Pin {
                    name: "2".into(),
                    role: Role::AnalogIn,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        i_max: 0.01.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
            ],
        }
    }
}

impl Block for Crystal {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

impl Inductor {
    /// Create a generic two-pin inductor.
    pub fn new(value: Qty<Henry>) -> Self {
        Self {
            value,
            pins: vec![
                Pin {
                    name: "1".into(),
                    role: Role::DigitalIO,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        i_max: 0.1.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
                Pin {
                    name: "2".into(),
                    role: Role::DigitalIO,
                    limits: Limits {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        i_max: 0.1.amp(),
                    },
                    sig: None,
                    pos: None,
                    rotation: None,
                    length: None,
                },
            ],
        }
    }
}

impl Block for Inductor {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacitor_new_has_two_digital_io_pins() {
        let c = Capacitor::new(100.0.nf());
        assert_eq!(c.pins().len(), 2);
        assert_eq!(c.pins()[0].name, "1");
        assert_eq!(c.pins()[1].name, "2");
        assert!(matches!(c.pins()[0].role, Role::DigitalIO));
        assert!(matches!(c.pins()[1].role, Role::DigitalIO));
        assert!(approx_eq(c.value.as_base(), 100e-9));
    }

    #[test]
    fn capacitor_decoupling_has_power_in_and_gnd() {
        let c = Capacitor::decoupling(10.0.uf());
        assert_eq!(c.pins().len(), 2);
        assert_eq!(c.pins()[0].name, "1");
        assert_eq!(c.pins()[1].name, "2");
        assert!(matches!(c.pins()[0].role, Role::PowerIn));
        assert!(matches!(c.pins()[1].role, Role::Gnd));
        assert!(approx_eq(c.pins()[0].limits.v_max.as_base(), 50.0));
    }

    #[test]
    fn resistor_new_has_two_pins() {
        let r = Resistor::new(10.0.kohm());
        assert_eq!(r.pins().len(), 2);
        assert!(matches!(r.pins()[0].role, Role::DigitalIO));
        assert!(matches!(r.pins()[1].role, Role::DigitalIO));
        assert!(approx_eq(r.value.as_base(), 10e3));
    }

    #[test]
    fn resistor_pullup_and_pulldown_have_digital_io_pins() {
        let pullup = Resistor::pullup(10.0.kohm(), "VCC");
        let pulldown = Resistor::pulldown(10.0.kohm(), "GND");
        assert_eq!(pullup.pins().len(), 2);
        assert_eq!(pulldown.pins().len(), 2);
        assert!(
            pullup
                .pins()
                .iter()
                .all(|p| matches!(p.role, Role::DigitalIO))
        );
        assert!(
            pulldown
                .pins()
                .iter()
                .all(|p| matches!(p.role, Role::DigitalIO))
        );
    }

    #[test]
    fn resistor_pullup_and_pulldown_store_net() {
        let pullup = Resistor::pullup(10.0.kohm(), "VCC");
        let pulldown = Resistor::pulldown(10.0.kohm(), "GND");
        assert_eq!(pullup.net, "VCC");
        assert_eq!(pulldown.net, "GND");
        assert_eq!(Resistor::new(1.0.kohm()).net, "");
    }

    #[test]
    fn crystal_new_has_two_analog_in_pins() {
        let y = Crystal::new(25.0.mhz());
        assert_eq!(y.pins().len(), 2);
        assert!(y.pins().iter().all(|p| matches!(p.role, Role::AnalogIn)));
        assert!((y.frequency.as_mhz() - 25.0).abs() < 1e-9);
    }

    #[test]
    fn inductor_new_has_two_pins_and_stores_value() {
        let l = Inductor::new(10.0e-6.henry());
        assert_eq!(l.pins().len(), 2);
        assert!(approx_eq(l.value.as_base(), 10e-6));
    }

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }
}
