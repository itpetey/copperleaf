//! Library of common parts used in examples and tests.

use copperleaf_model::{
    Component, Farad, Henry, Hertz, Ohm, Pin, PinBuilder, PinRef, PowerSpec, Qty, Role, UnitExt,
};

/// Standard two-pin capacitor.
#[derive(Clone, Debug)]
pub struct Capacitor {
    value: Qty<Farad>,
    pins: Vec<Pin>,
}

/// Standard two-pin resistor.
#[derive(Clone, Debug)]
pub struct Resistor {
    value: Qty<Ohm>,
    net: String,
    pins: Vec<Pin>,
}

/// Standard two-pin crystal.
#[derive(Clone, Debug)]
pub struct Crystal {
    frequency: Qty<Hertz>,
    pins: Vec<Pin>,
}

/// Standard two-pin inductor.
#[derive(Clone, Debug)]
pub struct Inductor {
    value: Qty<Henry>,
    pins: Vec<Pin>,
}

impl Capacitor {
    pub const PIN1: PinRef = PinRef("1");
    pub const PIN2: PinRef = PinRef("2");

    pub fn value(&self) -> Qty<Farad> {
        self.value
    }

    /// Create a generic two-pin capacitor with the given value.
    pub fn new(value: Qty<Farad>) -> Self {
        Self {
            value,
            pins: vec![
                Pin::build("1")
                    .role(Role::DigitalIO)
                    .power_spec(PowerSpec {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        v_nom: None,
                        i_max: 0.1.amp(),
                    })
                    .pin(),
                Pin::build("2")
                    .role(Role::DigitalIO)
                    .power_spec(PowerSpec {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        v_nom: None,
                        i_max: 0.1.amp(),
                    })
                    .pin(),
            ],
        }
    }

    /// Create a decoupling capacitor with PowerIn and Gnd pins rated for 50 V.
    pub fn decoupling(value: Qty<Farad>) -> Self {
        Self {
            value,
            pins: vec![
                Pin::build("1")
                    .role(Role::PowerIn)
                    .power_spec(PowerSpec {
                        v_min: 0.0.volt(),
                        v_max: 50.0.volt(),
                        v_nom: None,
                        i_max: 0.1.amp(),
                    })
                    .pin(),
                Pin::build("2").gnd(),
            ],
        }
    }
}

impl Component for Capacitor {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

impl Resistor {
    pub const PIN1: PinRef = PinRef("1");
    pub const PIN2: PinRef = PinRef("2");

    pub fn value(&self) -> Qty<Ohm> {
        self.value
    }

    pub fn net(&self) -> &str {
        &self.net
    }

    fn io_pin() -> PinBuilder {
        Pin::build("1").role(Role::DigitalIO).power_spec(PowerSpec {
            v_min: 0.0.volt(),
            v_max: 3.6.volt(),
            v_nom: None,
            i_max: 0.1.amp(),
        })
    }

    /// Create a generic two-pin resistor.
    pub fn new(value: Qty<Ohm>) -> Self {
        Self {
            value,
            net: String::new(),
            pins: vec![
                Self::io_pin().name("1").pin(),
                Self::io_pin().name("2").pin(),
            ],
        }
    }

    /// Create a pull-up resistor connected to `net`.
    pub fn pullup(value: Qty<Ohm>, net: &str) -> Self {
        Self {
            value,
            net: net.to_owned(),
            pins: vec![
                Self::io_pin().name("1").pin(),
                Self::io_pin().name("2").pin(),
            ],
        }
    }

    /// Create a pull-down resistor connected to `net`.
    pub fn pulldown(value: Qty<Ohm>, net: &str) -> Self {
        Self::pullup(value, net)
    }
}

impl Component for Resistor {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

impl Crystal {
    pub const PIN1: PinRef = PinRef("1");
    pub const PIN2: PinRef = PinRef("2");

    pub fn frequency(&self) -> Qty<Hertz> {
        self.frequency
    }

    /// Create a two-pin crystal with both pins as AnalogIn inputs.
    pub fn new(frequency: Qty<Hertz>) -> Self {
        Self {
            frequency,
            pins: vec![
                Pin::build("1")
                    .role(Role::AnalogIn)
                    .power_spec(PowerSpec {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        v_nom: None,
                        i_max: 0.01.amp(),
                    })
                    .pin(),
                Pin::build("2")
                    .role(Role::AnalogIn)
                    .power_spec(PowerSpec {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        v_nom: None,
                        i_max: 0.01.amp(),
                    })
                    .pin(),
            ],
        }
    }
}

impl Component for Crystal {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

impl Inductor {
    pub const PIN1: PinRef = PinRef("1");
    pub const PIN2: PinRef = PinRef("2");

    pub fn value(&self) -> Qty<Henry> {
        self.value
    }

    /// Create a generic two-pin inductor.
    pub fn new(value: Qty<Henry>) -> Self {
        Self {
            value,
            pins: vec![
                Pin::build("1")
                    .role(Role::DigitalIO)
                    .power_spec(PowerSpec {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        v_nom: None,
                        i_max: 0.1.amp(),
                    })
                    .pin(),
                Pin::build("2")
                    .role(Role::DigitalIO)
                    .power_spec(PowerSpec {
                        v_min: 0.0.volt(),
                        v_max: 3.6.volt(),
                        v_nom: None,
                        i_max: 0.1.amp(),
                    })
                    .pin(),
            ],
        }
    }
}

impl Component for Inductor {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn capacitor_new_has_two_digital_io_pins() {
        let c = Capacitor::new(100.0.nf());
        assert_eq!(c.pins().len(), 2);
        assert_eq!(c.pins()[0].name(), "1");
        assert_eq!(c.pins()[1].name(), "2");
        assert!(matches!(c.pins()[0].role(), Role::DigitalIO));
        assert!(matches!(c.pins()[1].role(), Role::DigitalIO));
    }

    #[test]
    fn capacitor_decoupling_has_power_in_and_gnd() {
        let c = Capacitor::decoupling(10.0.uf());
        assert_eq!(c.pins().len(), 2);
        assert_eq!(c.pins()[0].name(), "1");
        assert_eq!(c.pins()[1].name(), "2");
        assert!(matches!(c.pins()[0].role(), Role::PowerIn));
        assert!(matches!(c.pins()[1].role(), Role::Gnd));
        assert!((c.pins()[0].power_spec().v_max.as_base() - 50.0).abs() < 1e-9);
    }

    #[test]
    fn resistor_new_has_two_pins() {
        let r = Resistor::new(10.0.kohm());
        assert_eq!(r.pins().len(), 2);
        assert!(matches!(r.pins()[0].role(), Role::DigitalIO));
        assert!(matches!(r.pins()[1].role(), Role::DigitalIO));
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
                .all(|p| matches!(p.role(), Role::DigitalIO))
        );
        assert!(
            pulldown
                .pins()
                .iter()
                .all(|p| matches!(p.role(), Role::DigitalIO))
        );
        assert_eq!(pullup.net(), "VCC");
        assert_eq!(pulldown.net(), "GND");
    }

    #[test]
    fn crystal_new_has_two_analog_in_pins() {
        let y = Crystal::new(25.0.mhz());
        assert_eq!(y.pins().len(), 2);
        assert!(y.pins().iter().all(|p| matches!(p.role(), Role::AnalogIn)));
        assert!((y.frequency().as_mhz() - 25.0).abs() < 1e-9);
    }

    #[test]
    fn inductor_new_has_two_pins_and_stores_value() {
        let l = Inductor::new(10.0e-6.henry());
        assert_eq!(l.pins().len(), 2);
        assert!(approx_eq(l.value.as_base(), 10e-6));
    }

    #[test]
    fn constants_are_accessible() {
        assert_eq!(Capacitor::PIN1.0, "1");
        assert_eq!(Resistor::PIN2.0, "2");
        assert_eq!(Crystal::PIN1.0, "1");
        assert_eq!(Inductor::PIN2.0, "2");
    }
}
