//! Library of common parts used in examples and tests.
//!
//! These part models intentionally focus on pins and essential constraints only.

use copperleaf_core::{Amp, Qty, UnitExt, Volt};
use copperleaf_ir::*;

/// Simple synchronous buck regulator model with minimal pins and ratings.
#[derive(Clone, Debug)]
pub struct Buck {
    id: String,
    pins: Vec<Pin>,
    pub v_out: Qty<Volt>,
    pub i_max: Qty<Amp>,
}

/// Generic microcontroller model with USB pins and power rails.
#[derive(Clone, Debug)]
pub struct Mcu {
    id: String,
    pins: Vec<Pin>,
}

impl Buck {
    /// Create a new buck regulator with output voltage and current limit.
    pub fn new(id: &str, v_out: Qty<Volt>, i_max: Qty<Amp>) -> Self {
        Self {
            id: id.to_owned(),
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
                },
            ],
        }
    }
}

impl Block for Buck {
    fn id(&self) -> &str {
        &self.id
    }
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
    pub fn new(id: &str) -> Self {
        let usb_spec = SigSpec {
            kind: SigKind::Usb2Hs,
            bandwidth: Some(480.0.mhz()),
            edge_rate: None,
            target_impedance: Some(90.0.ohm()),
        };
        Self {
            id: id.to_owned(),
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
                },
            ],
        }
    }
}

impl Block for Mcu {
    fn id(&self) -> &str {
        &self.id
    }
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
