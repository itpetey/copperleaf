//! Library of common parts used in examples and tests.

use copperleaf::{
    Board, CompileError, Component, Farad, Hertz, Ohm, Pin, PinHandle, PinRef, PowerSpec, Qty,
    Role, UnitExt,
};
use copperleaf_part_macro::build_component;

use crate::footprint::Package;

pub mod footprint;

/// Standard two-pin capacitor.
///
/// The footprint is specified by a [`Package`] — use
/// [`Capacitor::new`] to create one with an SMD land pattern and KiCad
/// footprint reference.
#[derive(Clone, Debug)]
pub struct Capacitor {
    value: Qty<Farad>,
    pins: Vec<Pin>,
    footprint: Package,
}

/// Standard two-pin resistor.
///
/// The footprint is specified by a [`Package`] — use
/// [`Resistor::new`] to create one with an SMD land pattern and KiCad
/// footprint reference.
#[derive(Clone, Debug)]
pub struct Resistor {
    value: Qty<Ohm>,
    net: String,
    pins: Vec<Pin>,
    footprint: Package,
}

/// Standard two-pin crystal.
#[derive(Clone, Debug)]
pub struct Crystal {
    frequency: Qty<Hertz>,
    pins: Vec<Pin>,
}

impl Capacitor {
    pub const PIN1: PinRef = PinRef("1");
    pub const PIN2: PinRef = PinRef("2");

    pub fn value(&self) -> Qty<Farad> {
        self.value
    }

    fn smd_pin(name: &str, package: Package, index: usize) -> Pin {
        let lp = package.land_pattern();
        let x_offset = if index == 0 {
            -lp.pitch / 2.0
        } else {
            lp.pitch / 2.0
        };
        Pin::build(name)
            .role(Role::DigitalIO)
            .power_spec(PowerSpec {
                v_min: 0.0.volt(),
                v_max: 3.6.volt(),
                v_nom: None,
                i_max: 0.1.amp(),
            })
            .pos(x_offset, 0.0)
            .width(lp.pad_w)
            .height(lp.pad_h)
            .pad_type("smd")
            .pad_shape("rect")
            .layers("F.Cu F.Mask F.Paste")
            .pin()
    }

    /// Create a two-pin capacitor with the given value and SMD footprint package.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use copperleaf_parts_passives::{Capacitor, footprint};
    ///
    /// let c = Capacitor::new(100.0.nf(), Package::M1608);
    /// assert!(c.footprint().unwrap().contains("1608"));
    /// ```
    pub fn new(value: Qty<Farad>, package: Package) -> Self {
        Self {
            value,
            pins: vec![
                Self::smd_pin("1", package, 0),
                Self::smd_pin("2", package, 1),
            ],
            footprint: package,
        }
    }

    fn decoupling_pins(package: Package) -> Vec<Pin> {
        let lp = package.land_pattern();
        let x = lp.pitch / 2.0;
        vec![
            Pin::build("1")
                .role(Role::PowerIn)
                .power_spec(PowerSpec {
                    v_min: 0.0.volt(),
                    v_max: 50.0.volt(),
                    v_nom: None,
                    i_max: 0.1.amp(),
                })
                .pos(-x, 0.0)
                .width(lp.pad_w)
                .height(lp.pad_h)
                .pad_type("smd")
                .pad_shape("rect")
                .layers("F.Cu F.Mask F.Paste")
                .pin(),
            Pin::build("2")
                .role(Role::Gnd)
                .power_spec(PowerSpec {
                    v_min: 0.0.volt(),
                    v_max: 0.0.volt(),
                    v_nom: Some(0.0.volt()),
                    i_max: 100.0.amp(),
                })
                .pos(x, 0.0)
                .width(lp.pad_w)
                .height(lp.pad_h)
                .pad_type("smd")
                .pad_shape("rect")
                .layers("F.Cu F.Mask F.Paste")
                .pin(),
        ]
    }

    /// Create a decoupling capacitor with PowerIn and Gnd pins rated for 50 V.
    pub fn decoupling(value: Qty<Farad>, package: Package) -> Self {
        Self {
            value,
            pins: Self::decoupling_pins(package),
            footprint: package,
        }
    }
}

impl Component for Capacitor {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }

    fn footprint(&self) -> Option<&'static str> {
        Some(self.footprint.capacitor_footprint_name())
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

    fn smd_pin(name: &str, package: Package, index: usize) -> Pin {
        let lp = package.land_pattern();
        let x_offset = if index == 0 {
            -lp.pitch / 2.0
        } else {
            lp.pitch / 2.0
        };
        Pin::build(name)
            .role(Role::DigitalIO)
            .power_spec(PowerSpec {
                v_min: 0.0.volt(),
                v_max: 3.6.volt(),
                v_nom: None,
                i_max: 0.1.amp(),
            })
            .pos(x_offset, 0.0)
            .width(lp.pad_w)
            .height(lp.pad_h)
            .pad_type("smd")
            .pad_shape("rect")
            .layers("F.Cu F.Mask F.Paste")
            .pin()
    }

    /// Create a two-pin resistor with the given value and SMD footprint package.
    pub fn new(value: Qty<Ohm>, package: Package) -> Self {
        Self {
            value,
            net: String::new(),
            pins: vec![
                Self::smd_pin("1", package, 0),
                Self::smd_pin("2", package, 1),
            ],
            footprint: package,
        }
    }

    /// Create a pull-up resistor connected to `net`.
    pub fn pullup(value: Qty<Ohm>, net: &str, package: Package) -> Self {
        Self {
            value,
            net: net.to_owned(),
            pins: vec![
                Self::smd_pin("1", package, 0),
                Self::smd_pin("2", package, 1),
            ],
            footprint: package,
        }
    }

    /// Create a pull-down resistor connected to `net`.
    pub fn pulldown(value: Qty<Ohm>, net: &str, package: Package) -> Self {
        Self::pullup(value, net, package)
    }
}

impl Component for Resistor {
    fn pins(&self) -> &[Pin] {
        &self.pins
    }

    fn footprint(&self) -> Option<&'static str> {
        Some(self.footprint.resistor_footprint_name())
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

build_component!("b82472p6152m000.toml");

build_component!("b82472p6222m000.toml");

/// Add a pull-down resistor from `pin` to the given ground pin.
pub fn pulldown(
    board: &mut Board,
    refdes: &str,
    pin: PinHandle,
    gnd: PinHandle,
    package: Package,
) -> Result<(), CompileError> {
    let r = board.add(refdes, Resistor::new(10.0.kohm(), package));
    board.connect(pin, r.pin(Resistor::PIN1))?;
    board.connect(gnd, r.pin(Resistor::PIN2))?;
    Ok(())
}

/// Add a pull-up resistor from `pin` to the `vdd_pin` power pin.
pub fn pullup(
    board: &mut Board,
    refdes: &str,
    pin: PinHandle,
    vdd_pin: PinHandle,
    package: Package,
) -> Result<(), CompileError> {
    let r = board.add(refdes, Resistor::new(10.0.kohm(), package));
    board.connect(pin, r.pin(Resistor::PIN1))?;
    board.connect(vdd_pin, r.pin(Resistor::PIN2))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacitor_new_has_two_digital_io_pins() {
        let c = Capacitor::new(100.0.nf(), Package::M1608);
        assert_eq!(c.pins().len(), 2);
        assert_eq!(c.pins()[0].name(), "1");
        assert_eq!(c.pins()[1].name(), "2");
        assert!(matches!(c.pins()[0].role(), Role::DigitalIO));
        assert!(matches!(c.pins()[1].role(), Role::DigitalIO));
    }

    #[test]
    fn capacitor_decoupling_has_power_in_and_gnd() {
        let c = Capacitor::decoupling(10.0.uf(), Package::M1608);
        assert_eq!(c.pins().len(), 2);
        assert_eq!(c.pins()[0].name(), "1");
        assert_eq!(c.pins()[1].name(), "2");
        assert!(matches!(c.pins()[0].role(), Role::PowerIn));
        assert!(matches!(c.pins()[1].role(), Role::Gnd));
        assert!((c.pins()[0].power_spec().v_max.as_base() - 50.0).abs() < 1e-9);
    }

    #[test]
    fn resistor_new_has_two_pins() {
        let r = Resistor::new(10.0.kohm(), Package::M1608);
        assert_eq!(r.pins().len(), 2);
        assert!(matches!(r.pins()[0].role(), Role::DigitalIO));
        assert!(matches!(r.pins()[1].role(), Role::DigitalIO));
    }

    #[test]
    fn resistor_pullup_and_pulldown_have_digital_io_pins() {
        let pullup = Resistor::pullup(10.0.kohm(), "VCC", Package::M1608);
        let pulldown = Resistor::pulldown(10.0.kohm(), "GND", Package::M1608);
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
    fn constants_are_accessible() {
        assert_eq!(Capacitor::PIN1.0, "1");
        assert_eq!(Resistor::PIN2.0, "2");
        assert_eq!(Crystal::PIN1.0, "1");
    }

    #[test]
    fn capacitor_new_sets_footprint_and_pad_geometry() {
        let c = Capacitor::new(100.0.nf(), Package::M1608);
        let fp = c.footprint().unwrap();
        assert!(fp.contains("0603") || fp.contains("1608"));
        for i in 0..2 {
            assert!(c.pins()[i].pos().is_some());
            assert_eq!(c.pins()[i].pad_type(), Some("smd"));
            assert!(c.pins()[i].width().unwrap() > 0.0);
            assert!(c.pins()[i].height().unwrap() > 0.0);
        }
        // Pad 1 and 2 should be on opposite sides of the origin
        let x0 = c.pins()[0].pos().unwrap().0;
        let x1 = c.pins()[1].pos().unwrap().0;
        assert!(x0 < 0.0, "pad 1 should be left of origin");
        assert!(x1 > 0.0, "pad 2 should be right of origin");
    }

    #[test]
    fn capacitor_decoupling_sets_footprint() {
        let c = Capacitor::decoupling(10.0.uf(), Package::M2012);
        assert!(c.footprint().is_some());
        for pin in c.pins() {
            assert_eq!(pin.pad_type(), Some("smd"));
            assert!(pin.pos().is_some());
        }
    }

    #[test]
    fn resistor_new_sets_footprint_and_geometry() {
        let r = Resistor::new(10.0.kohm(), Package::M1005);
        let fp = r.footprint().unwrap();
        assert!(fp.contains("0402") || fp.contains("1005"));
        for pin in r.pins() {
            assert_eq!(pin.pad_type(), Some("smd"));
            assert!(pin.width().unwrap() > 0.0);
        }
    }

    #[test]
    fn resistor_pullup_sets_footprint() {
        let r = Resistor::pullup(10.0.kohm(), "VCC", Package::M3216);
        assert!(r.footprint().is_some());
        assert_eq!(r.net(), "VCC");
    }

    #[test]
    fn resistor_pulldown_sets_footprint() {
        let r = Resistor::pulldown(10.0.kohm(), "GND", Package::M3216);
        assert!(r.footprint().is_some());
        assert_eq!(r.net(), "GND");
    }

    #[test]
    fn all_footprints_produce_valid_geometry() {
        for package in [
            Package::M0603,
            Package::M1005,
            Package::M1608,
            Package::M2012,
            Package::M3216,
            Package::M3225,
            Package::M4532,
            Package::M5025,
            Package::M6332,
        ] {
            let r = Resistor::new(10.0.kohm(), package);
            assert!(r.footprint().is_some());
            for pin in r.pins() {
                assert_eq!(pin.pad_type(), Some("smd"));
                assert!(pin.pos().is_some());
            }
        }
    }

    #[test]
    fn capacitor_with_package_still_has_correct_roles() {
        let c = Capacitor::new(100.0.nf(), Package::M1608);
        assert!(matches!(c.pins()[0].role(), Role::DigitalIO));
        assert!(matches!(c.pins()[1].role(), Role::DigitalIO));
    }
}
