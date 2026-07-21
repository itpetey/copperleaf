/// JST PH Connector Header Surface Mount, Right Angle 2 position 2.00mm
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | 1        | I/O         |                       |
/// | 1   | SHIELD   | I/O         |                       |
/// | 2   | 2        | I/O         |                       |
/// | 2   | SHIELD   | I/O         |                       |
pub struct S2bPhSm4TbLfSn {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::MechanicalPad>,
}

impl S2bPhSm4TbLfSn {
    pub const PIN_1: copperleaf::PinRef = copperleaf::PinRef("1");
    pub const SHIELD: copperleaf::PinRef = copperleaf::PinRef("SHIELD");
    pub const PIN_2: copperleaf::PinRef = copperleaf::PinRef("2");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("1").number("1").pos(-1.0, 0.0).rotation(0.0).length(3.5).width(1.0).height(3.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("SHIELD").number("S1").pos(-3.35, 5.75).rotation(0.0).length(3.4).width(1.5).height(3.4).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("2").number("2").pos(1.0, 0.0).rotation(0.0).length(3.5).width(1.0).height(3.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("SHIELD").number("S2").pos(3.35, 5.75).rotation(0.0).length(3.4).width(1.5).height(3.4).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
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

impl copperleaf::Component for S2bPhSm4TbLfSn {
    fn pins(&self) -> &[copperleaf::Pin] {
        &self.pins
    }

    fn constraints(&self) -> Vec<copperleaf::Constraint> {
        Self::constraints(self)
    }

    fn mechanical(&self) -> &[copperleaf::MechanicalPad] {
        &self.mechanical
    }

    fn symbol(&self) -> Option<&'static str> {
        Some("S2B-PH-SM4-TB_LF__SN_")
    }

    fn footprint(&self) -> Option<&'static str> {
        Some("S2B-PH-SM4-TB_LF__SN_")
    }

    fn model_3d_data(&self) -> Option<&'static str> {
        Some("<elided:504344:6f835a50f9b8e9d0>")
    }

    fn model_3d_rotation(&self) -> (f64, f64, f64) {
        (90.0, 0.0, 0.0)
    }

    fn model_3d_offset(&self) -> (f64, f64, f64) {
        (0.0, -4.5, 0.0)
    }
}

impl Default for S2bPhSm4TbLfSn {
    fn default() -> Self {
        Self::new()
    }
}
