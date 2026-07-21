/// TE Connectivity USB Type C 2.0 Receptacle, Mid-Mount, Right Angle
///
/// Datasheet: https://www.te.com/commerce/DocumentDelivery/DDEController?Action=srchrtrv&DocNm=2340901&DocType=Customer+Drawing&DocLang=English&PartCntxt=2340901-1&DocFormat=pdf
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | GND_A    | Ground      |                       |
/// | 2   | VBUS_A   | Supply      |                       |
/// | 3   | DP1      | I/O         |                       |
/// | 4   | CC1      | I/O         |                       |
/// | 5   | SBU1     | I/O         |                       |
/// | 6   | DN1      | I/O         |                       |
/// | 7   | SHIELD   | I/O         |                       |
/// | 8   | SHIELD   | I/O         |                       |
/// | 9   | SHIELD   | I/O         |                       |
/// | 10  | SHIELD   | I/O         |                       |
/// | 11  | GND_B    | Ground      |                       |
/// | 12  | VBUS_B   | Supply      |                       |
/// | 13  | DP2      | I/O         |                       |
/// | 14  | CC2      | I/O         |                       |
/// | 15  | SBU2     | I/O         |                       |
/// | 16  | DN2      | I/O         |                       |
pub struct UsbC23409011 {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl UsbC23409011 {
    pub const GND_A: copperleaf::PinRef = copperleaf::PinRef("GND_A");
    pub const VBUS_A: copperleaf::PinRef = copperleaf::PinRef("VBUS_A");
    pub const DP1: copperleaf::PinRef = copperleaf::PinRef("DP1");
    pub const CC1: copperleaf::PinRef = copperleaf::PinRef("CC1");
    pub const SBU1: copperleaf::PinRef = copperleaf::PinRef("SBU1");
    pub const DN1: copperleaf::PinRef = copperleaf::PinRef("DN1");
    pub const SHIELD: copperleaf::PinRef = copperleaf::PinRef("SHIELD");
    pub const GND_B: copperleaf::PinRef = copperleaf::PinRef("GND_B");
    pub const VBUS_B: copperleaf::PinRef = copperleaf::PinRef("VBUS_B");
    pub const DP2: copperleaf::PinRef = copperleaf::PinRef("DP2");
    pub const CC2: copperleaf::PinRef = copperleaf::PinRef("CC2");
    pub const SBU2: copperleaf::PinRef = copperleaf::PinRef("SBU2");
    pub const DN2: copperleaf::PinRef = copperleaf::PinRef("DN2");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("GND_A").number("A1/B12").pos(-3.2, -5.275).rotation(0.0).length(1.15).width(0.6).height(1.15).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("VBUS_A").number("A4/B9").pos(-2.4, -5.275).rotation(0.0).length(1.15).width(0.6).height(1.15).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(4.75.volt(), 5.25.volt(), 3.0.amp()).pin(),
                Pin::build("DP1").number("A6").pos(-0.25, -5.275).rotation(0.0).length(1.15).width(0.3).height(1.15).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("CC1").number("A5").pos(-1.25, -5.275).rotation(0.0).length(1.15).width(0.3).height(1.15).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("SBU1").number("A8").pos(1.25, -5.275).rotation(0.0).length(1.15).width(0.3).height(1.15).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("DN1").number("A7").pos(0.25, -5.275).rotation(0.0).length(1.15).width(0.3).height(1.15).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("SHIELD").number("SH1").pos(-5.62, -4.0).rotation(0.0).length(1.85).width(1.05).height(1.85).pad_type("thru_hole").pad_shape("oval").solder_mask_margin(0.102).layers("*.Cu *.Mask").dio(),
                Pin::build("SHIELD").number("SH2").pos(5.62, -4.0).rotation(0.0).length(1.85).width(1.05).height(1.85).pad_type("thru_hole").pad_shape("oval").solder_mask_margin(0.102).layers("*.Cu *.Mask").dio(),
                Pin::build("SHIELD").number("SH3").pos(-5.62, 0.0).rotation(0.0).length(2.25).width(1.05).height(2.25).pad_type("thru_hole").pad_shape("oval").solder_mask_margin(0.102).layers("*.Cu *.Mask").dio(),
                Pin::build("SHIELD").number("SH4").pos(5.62, 0.0).rotation(0.0).length(2.25).width(1.05).height(2.25).pad_type("thru_hole").pad_shape("oval").solder_mask_margin(0.102).layers("*.Cu *.Mask").dio(),
                Pin::build("GND_B").number("B1/A12").pos(3.2, -5.275).rotation(0.0).length(1.15).width(0.6).height(1.15).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("VBUS_B").number("B4/AP").pos(2.4, -5.275).rotation(0.0).length(1.15).width(0.6).height(1.15).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(4.75.volt(), 5.25.volt(), 3.0.amp()).pin(),
                Pin::build("DP2").number("B6").pos(0.75, -5.275).rotation(0.0).length(1.15).width(0.3).height(1.15).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("CC2").number("B5").pos(1.75, -5.275).rotation(0.0).length(1.15).width(0.3).height(1.15).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("SBU2").number("B8").pos(-1.75, -5.275).rotation(0.0).length(1.15).width(0.3).height(1.15).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("DN2").number("B7").pos(-0.75, -5.275).rotation(0.0).length(1.15).width(0.3).height(1.15).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
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

impl copperleaf::Component for UsbC23409011 {
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
        Some("2340901-1")
    }

    fn footprint(&self) -> Option<&'static str> {
        Some("2340901-1")
    }

    fn datasheet(&self) -> Option<&'static str> {
        Some("https://www.te.com/commerce/DocumentDelivery/DDEController?Action=srchrtrv&DocNm=2340901&DocType=Customer+Drawing&DocLang=English&PartCntxt=2340901-1&DocFormat=pdf")
    }

    fn model_3d_data(&self) -> Option<&'static str> {
        Some("<elided:1093848:67f8b24c39453ba8>")
    }

    fn model_3d_rotation(&self) -> (f64, f64, f64) {
        (-90.0, 0.0, 0.0)
    }

    fn model_3d_offset(&self) -> (f64, f64, f64) {
        (0.0, -4.5, 0.0)
    }
}

impl Default for UsbC23409011 {
    fn default() -> Self {
        Self::new()
    }
}
