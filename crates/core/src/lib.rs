//! Core types for Copperleaf.
//!
//! This crate exposes strongly-typed quantities backed by `uom`, basic
//! diagnostic types, and simple identifier newtypes. These building blocks are
//! re-exported by the `copperleaf` facade crate for downstream use.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::marker::PhantomData;
use uom::si::electric_potential::volt;
use uom::si::electrical_resistance::ohm;
use uom::si::f64 as uq;
use uom::si::length::meter;
use uom::si::thermodynamic_temperature::degree_celsius;
use uom::si::time::second;

/// Marker trait used to bridge concrete `uom` quantities with a simple
/// generic wrapper [`Qty`]. Each marker defines its underlying `uom` type,
/// a human label, and conversions to/from the base unit.
pub trait UnitMarker {
    type Q;
    const LABEL: &'static str;
    fn to_base(q: &Self::Q) -> f64;
    fn from_base(v: f64) -> Self::Q;
}

/// Electric potential (volts)
#[derive(Clone, Copy, Debug)]
pub struct Volt;
/// Electric current (amperes)
#[derive(Clone, Copy, Debug)]
pub struct Amp;
/// Electrical resistance (ohms)
#[derive(Clone, Copy, Debug)]
pub struct Ohm;
/// Capacitance (farads)
#[derive(Clone, Copy, Debug)]
pub struct Farad;
/// Length (meters)
#[derive(Clone, Copy, Debug)]
pub struct Meter;
/// Time (seconds)
#[derive(Clone, Copy, Debug)]
pub struct Second;
/// Temperature (degrees Celsius)
#[derive(Clone, Copy, Debug)]
pub struct Celsius;

impl UnitMarker for Volt {
    type Q = uq::ElectricPotential;
    const LABEL: &'static str = "V";
    fn to_base(q: &Self::Q) -> f64 {
        q.get::<volt>()
    }
    fn from_base(v: f64) -> Self::Q {
        uq::ElectricPotential::new::<volt>(v)
    }
}
impl UnitMarker for Amp {
    type Q = uq::ElectricCurrent;
    const LABEL: &'static str = "A";
    fn to_base(q: &Self::Q) -> f64 {
        q.value
    }
    fn from_base(v: f64) -> Self::Q {
        uq::ElectricCurrent::new::<uom::si::electric_current::ampere>(v)
    }
}
impl UnitMarker for Ohm {
    type Q = uq::ElectricalResistance;
    const LABEL: &'static str = "Ohm";
    fn to_base(q: &Self::Q) -> f64 {
        q.get::<ohm>()
    }
    fn from_base(v: f64) -> Self::Q {
        uq::ElectricalResistance::new::<ohm>(v)
    }
}
impl UnitMarker for Farad {
    type Q = uq::Capacitance;
    const LABEL: &'static str = "F";
    fn to_base(q: &Self::Q) -> f64 {
        q.value
    }
    fn from_base(v: f64) -> Self::Q {
        uq::Capacitance::new::<uom::si::capacitance::farad>(v)
    }
}
impl UnitMarker for Meter {
    type Q = uq::Length;
    const LABEL: &'static str = "m";
    fn to_base(q: &Self::Q) -> f64 {
        q.get::<meter>()
    }
    fn from_base(v: f64) -> Self::Q {
        uq::Length::new::<meter>(v)
    }
}
impl UnitMarker for Second {
    type Q = uq::Time;
    const LABEL: &'static str = "s";
    fn to_base(q: &Self::Q) -> f64 {
        q.get::<second>()
    }
    fn from_base(v: f64) -> Self::Q {
        uq::Time::new::<second>(v)
    }
}
impl UnitMarker for Celsius {
    type Q = uq::ThermodynamicTemperature;
    const LABEL: &'static str = "C";
    fn to_base(q: &Self::Q) -> f64 {
        q.get::<degree_celsius>()
    }
    fn from_base(v: f64) -> Self::Q {
        uq::ThermodynamicTemperature::new::<degree_celsius>(v)
    }
}

/// Generic quantity wrapper parameterized by a [`UnitMarker`].
///
/// Values serialize as `{ value, unit }` in base units for stable JSON.
#[derive(Clone, Copy, Debug)]
pub struct Qty<U: UnitMarker>(pub U::Q, pub PhantomData<U>);

impl<U: UnitMarker> Qty<U> {
    /// Returns the value in the base unit as an `f64` for display or simple calculations.
    pub fn as_base(&self) -> f64 {
        U::to_base(&self.0)
    }
}

impl<U: UnitMarker> Serialize for Qty<U> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Helper<'a> {
            value: f64,
            unit: &'a str,
        }
        let h = Helper {
            value: self.as_base(),
            unit: U::LABEL,
        };
        h.serialize(serializer)
    }
}

impl<'de, U: UnitMarker> Deserialize<'de> for Qty<U> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Helper {
            value: f64,
            _unit: Option<String>,
        }
        let h = Helper::deserialize(deserializer)?;
        let q = U::from_base(h.value);
        Ok(Qty(q, PhantomData))
    }
}

/// Extension methods on numeric types to construct typed quantities.
pub trait UnitExt {
    /// Construct a voltage in volts.
    fn volt(self) -> Qty<Volt>;
    /// Construct a voltage in millivolts.
    fn millivolt(self) -> Qty<Volt>;
    /// Construct a current in amperes.
    fn amp(self) -> Qty<Amp>;
    /// Constructs a current in milliamperes.
    fn milliamp(self) -> Qty<Amp>;
    /// Construct a resistance in ohms.
    fn ohm(self) -> Qty<Ohm>;
    /// Construct a resistance in kilo-ohms.
    fn kohm(self) -> Qty<Ohm>;
    /// Construct a capacitance in farads.
    fn farad(self) -> Qty<Farad>;
    /// Construct a length in millimeters (converted to meters internally).
    fn mm(self) -> Qty<Meter>;
    /// Construct a length in meters.
    fn meter(self) -> Qty<Meter>;
    /// Construct a time in seconds.
    fn sec(self) -> Qty<Second>;
    /// Construct a temperature in degrees Celsius.
    fn celsius(self) -> Qty<Celsius>;
    /// Construct a period from a frequency in megahertz (returns seconds per cycle).
    fn mhz(self) -> Qty<Second>;
    /// Construct a capacitance in nanofarads.
    fn nf(self) -> Qty<Farad>;
    /// Construct a capacitance in microfarads.
    fn uf(self) -> Qty<Farad>;
    /// Construct a capacitance in picofarads.
    fn pf(self) -> Qty<Farad>;
}

impl UnitExt for f64 {
    fn volt(self) -> Qty<Volt> {
        Qty(Volt::from_base(self), PhantomData)
    }
    fn millivolt(self) -> Qty<Volt> {
        Qty(Volt::from_base(self * 1.0e-3), PhantomData)
    }
    fn amp(self) -> Qty<Amp> {
        Qty(Amp::from_base(self), PhantomData)
    }
    fn milliamp(self) -> Qty<Amp> {
        Qty(Amp::from_base(self * 1.0e-3), PhantomData)
    }
    fn ohm(self) -> Qty<Ohm> {
        Qty(Ohm::from_base(self), PhantomData)
    }
    fn kohm(self) -> Qty<Ohm> {
        Qty(Ohm::from_base(self * 1.0e3), PhantomData)
    }
    fn farad(self) -> Qty<Farad> {
        Qty(Farad::from_base(self), PhantomData)
    }
    fn mm(self) -> Qty<Meter> {
        Qty(Meter::from_base(self / 1000.0), PhantomData)
    }
    fn meter(self) -> Qty<Meter> {
        Qty(Meter::from_base(self), PhantomData)
    }
    fn sec(self) -> Qty<Second> {
        Qty(Second::from_base(self), PhantomData)
    }
    fn celsius(self) -> Qty<Celsius> {
        Qty(Celsius::from_base(self), PhantomData)
    }
    fn mhz(self) -> Qty<Second> {
        // period in seconds
        let hz = self * 1.0e6;
        Qty(Second::from_base(1.0 / hz), PhantomData)
    }
    fn nf(self) -> Qty<Farad> {
        Qty(Farad::from_base(self * 1.0e-9), PhantomData)
    }
    fn uf(self) -> Qty<Farad> {
        Qty(Farad::from_base(self * 1.0e-6), PhantomData)
    }
    fn pf(self) -> Qty<Farad> {
        Qty(Farad::from_base(self * 1.0e-12), PhantomData)
    }
}

/// Diagnostic severity for analysis and verification messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// A structured diagnostic produced by analysis or backends.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Short stable code, e.g. `ERC:OVERVOLT`.
    pub code: String,
    /// Message severity.
    pub severity: Severity,
    /// Human-readable summary.
    pub message: String,
    /// Entity identifiers related to the message (e.g., nets or pins).
    pub entities: Vec<String>,
    /// Optional hint with a suggested fix.
    pub hint: Option<String>,
}

// Stable IDs (string newtypes for now)
/// Identifier for a component instance (e.g., `U1`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComponentId(pub String);
/// Identifier for a net name.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetId(pub String);
/// Identifier for a specific pin on a component.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PinId(pub String);
/// Identifier for a constraint instance.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConstraintId(pub String);
