/// JST PH Connector Header Surface Mount, Right Angle 2 position 2.00mm
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | 1        | I/O         |                       |
/// | 1   | SHIELD_1 | I/O         |                       |
/// | 2   | 2        | I/O         |                       |
/// | 2   | SHIELD_2 | I/O         |                       |
pub struct S2bPhSm4TbLfSn {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl S2bPhSm4TbLfSn {
    pub const PIN_1: copperleaf::PinRef = copperleaf::PinRef("1");
    pub const SHIELD_1: copperleaf::PinRef = copperleaf::PinRef("SHIELD_1");
    pub const PIN_2: copperleaf::PinRef = copperleaf::PinRef("2");
    pub const SHIELD_2: copperleaf::PinRef = copperleaf::PinRef("SHIELD_2");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("1").number("1").pos(-1.0, 0.0).rotation(0.0).length(3.5).width(1.0).height(3.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("SHIELD_1").number("S1").pos(-3.35, 5.75).rotation(0.0).length(3.4).width(1.5).height(3.4).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("2").number("2").pos(1.0, 0.0).rotation(0.0).length(3.5).width(1.0).height(3.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("SHIELD_2").number("S2").pos(3.35, 5.75).rotation(0.0).length(3.4).width(1.5).height(3.4).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
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

    fn mechanical(&self) -> &[copperleaf::Pad] {
        &self.mechanical
    }

    fn meta(&self) -> &copperleaf::ComponentMeta {
        static META: std::sync::OnceLock<copperleaf::ComponentMeta> = std::sync::OnceLock::new();
        META.get_or_init(|| copperleaf::ComponentMeta {
            symbol: Some("S2B-PH-SM4-TB_LF__SN_".into()),
            
            footprint: Some("S2B-PH-SM4-TB_LF__SN_".into()),
            
            
            datasheet: None,
            
            description: None,
            
            model_3d: None,
            model_3d_data: Some("<elided:504344:6f835a50f9b8e9d0>".into()),
            
            model_3d_rotation: (-90.0, 0.0, 0.0),
            
            model_3d_offset: (0.0, -4.5, 0.0),
            
            
            fab_extent: None,
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for S2bPhSm4TbLfSn {
    fn default() -> Self {
        Self::new()
    }
}
