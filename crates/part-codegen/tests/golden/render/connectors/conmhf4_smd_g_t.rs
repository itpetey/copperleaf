/// TE Connectivity MHF4 Coaxial Connector, 50 Ohm Snap-On, 6 GHz (SMD)
///
/// Datasheet: https://www.te.com/commerce/DocumentDelivery/DDEController?Action=srchrtrv&DocNm=conmhf4-smd-g-t-ds&DocType=Data%20Sheet&DocLang=English&DocFormat=pdf&PartCntxt=CONMHF4-SMD-G-T
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | Signal   | RF Signal   | Center contact (male pin), gold-plated brass. Contact resistance 20.0 mohm max. Insertion loss: 0.33 dB (400-960 MHz), 0.43 dB (1164-1609 MHz), 0.58 dB (2.4 GHz), 1.60 dB (1427-5000 MHz). |
/// | 2   | GND1     | Ground      | Outer contact, gold-plated brass. Contact resistance 20.0 mohm max. |
/// | 3   | GND2     | Ground      | Outer contact, gold-plated brass. Contact resistance 20.0 mohm max. |
/// | 4   | GND3     | Ground      | Outer contact, gold-plated brass. Contact resistance 20.0 mohm max. |
pub struct Conmhf4SmdGT {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Conmhf4SmdGT {
    pub const Signal: copperleaf::PinRef = copperleaf::PinRef("Signal");
    pub const GND1: copperleaf::PinRef = copperleaf::PinRef("GND1");
    pub const GND2: copperleaf::PinRef = copperleaf::PinRef("GND2");
    pub const GND3: copperleaf::PinRef = copperleaf::PinRef("GND3");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("Signal").number("1").pos(0.0, 0.95).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").role(Role::AnalogIn).rf_limits().pin(),
                Pin::build("GND1").number("2").pos(-0.96, 0.0).rotation(0.0).length(1.58).width(0.58).height(1.58).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("GND2").number("3").pos(0.96, 0.0).rotation(0.0).length(1.58).width(0.58).height(1.58).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("GND3").number("4").pos(0.0, -1.0).rotation(0.0).length(0.5).width(0.5).height(0.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
            ],
            mechanical: vec![
            ],
        }
    }

    pub fn constraints(&self) -> Vec<copperleaf::Constraint> {
        use copperleaf::{Constraint, units::UnitExt};
        vec![
        ]
    }
}

impl copperleaf::Component for Conmhf4SmdGT {
    fn pins(&self) -> &[copperleaf::Pin] {
        &self.pins
    }

    fn constraints(&self) -> Vec<copperleaf::Constraint> {
        Self::constraints(self)
    }

    fn mechanical(&self) -> &[copperleaf::Pad] {
        &self.mechanical
    }

    fn symbol(&self) -> Option<&'static str> {
        Some("CONMHF4-SMD-G-T")
    }

    fn footprint(&self) -> Option<&'static str> {
        Some("CONMHF4-SMD-G-T")
    }

    fn datasheet(&self) -> Option<&'static str> {
        Some("https://www.te.com/commerce/DocumentDelivery/DDEController?Action=srchrtrv&DocNm=conmhf4-smd-g-t-ds&DocType=Data%20Sheet&DocLang=English&DocFormat=pdf&PartCntxt=CONMHF4-SMD-G-T")
    }

    fn model_3d_data(&self) -> Option<&'static str> {
        Some("<elided:361652:8b1b54454b3624c8>")
    }

    fn model_3d_rotation(&self) -> (f64, f64, f64) {
        (90.0, 0.0, 0.0)
    }
}

impl Default for Conmhf4SmdGT {
    fn default() -> Self {
        Self::new()
    }
}
