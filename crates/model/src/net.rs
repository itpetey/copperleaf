use crate::pin::SigSpec;
use crate::units::{Farad, Meter, Ohm, Qty, UnitExt, Volt};

/// Identifier for a net name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NetId(pub String);

#[derive(Clone, Debug)]
pub enum NetKind {
    Power {
        v_nom: Qty<Volt>,
        ripple: Option<Qty<Volt>>,
    },
    Signal {
        spec: SigSpec,
    },
}

#[derive(Clone, Debug, Default)]
pub struct NetClass {
    pub min_width: Option<Qty<Meter>>,
    pub clearance: Option<Qty<Meter>>,
}

#[derive(Clone, Debug)]
pub enum Constraint {
    Impedance {
        target: Qty<Ohm>,
        tol_pct: f64,
    },
    LengthMatch {
        group: String,
        skew_ps: f64,
    },
    ReturnPath {
        requires_plane: bool,
    },
    NetClass {
        min_width: Qty<Meter>,
        clearance: Qty<Meter>,
    },
    Creepage {
        min: Qty<Meter>,
        voltage: Qty<Volt>,
    },
    Decoupling {
        values: Vec<Qty<Farad>>,
        per_pin: bool,
    },
    ResonanceIndex {
        max: f64,
    },
    MaxJunction {
        temp: Qty<Celsius>,
    },
}

use crate::units::Celsius;

#[derive(Clone, Debug)]
pub struct Net {
    pub name: String,
    pub kind: NetKind,
    pub class: NetClass,
    pub constraints: Vec<Constraint>,
}

/// Handle to an emerging net, returned by [`Board::connect`](crate::Board::connect).
#[derive(Clone, Copy, Debug)]
pub struct NetHandle {
    pub(crate) edge: usize,
}

impl Net {
    /// Create a power net with nominal voltage.
    pub fn power(name: &str, v_nom: Qty<Volt>) -> Self {
        Self {
            name: name.to_string(),
            kind: NetKind::Power {
                v_nom,
                ripple: None,
            },
            class: NetClass::default(),
            constraints: vec![],
        }
    }

    /// Convenience constructor for a ground net named `GND`.
    pub fn ground() -> Self {
        Self::power("GND", 0.0.volt())
    }

    /// Set allowed ripple for a power net.
    pub fn ripple(mut self, r: Qty<Volt>) -> Self {
        if let NetKind::Power { v_nom, .. } = self.kind {
            self.kind = NetKind::Power {
                v_nom,
                ripple: Some(r),
            };
        }
        self
    }
}
