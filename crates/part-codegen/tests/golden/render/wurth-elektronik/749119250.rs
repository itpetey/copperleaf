/// Würth Elektronik 749119250 — Power Over Ethernet (PoE) For For DC/DC Converters SMPS Transformer 1.5kVAC Isolation 200 ~ 500kHz Surface Mount (ER14.5/6)
///
/// Datasheet: https://www.we-online.com/components/products/datasheet/749119250.pdf
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | ~        | I/O         | Primary winding N1 terminal (input, Vin 36-57 V DC) |
/// | 3   | ~        | I/O         | Primary winding N1 terminal (input, Vin 36-57 V DC) |
/// | 5   | ~        | I/O         | Auxiliary winding N5 terminal (Vaux 9 V) |
/// | 6   | ~        | I/O         | Auxiliary winding N5 terminal (Vaux 9 V) |
/// | 7   | ~        | I/O         | Secondary winding N3 terminal (Output 2, VOut2 5 V / IOut2 0.47 A) |
/// | 8   | ~        | I/O         | Secondary winding N2 terminal (Output 1, VOut1 5 V / IOut1 0.47 A) |
/// | 9   | ~        | I/O         | Secondary winding N4 terminal (Output 3, VOut3 5 V / IOut3 0.47 A) |
/// | 10  | ~        | I/O         | Secondary winding N4 terminal (Output 3, VOut3 5 V / IOut3 0.47 A) |
/// | 11  | ~        | I/O         | Secondary winding N3 terminal (Output 2, VOut2 5 V / IOut2 0.47 A) |
/// | 12  | ~        | I/O         | Secondary winding N2 terminal (Output 1, VOut1 5 V / IOut1 0.47 A) |
pub struct Comp749119250 {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Comp749119250 {
    pub const _PIN_1: copperleaf::PinRef = copperleaf::PinRef("1");
    pub const _PIN_3: copperleaf::PinRef = copperleaf::PinRef("3");
    pub const _PIN_5: copperleaf::PinRef = copperleaf::PinRef("5");
    pub const _PIN_6: copperleaf::PinRef = copperleaf::PinRef("6");
    pub const _PIN_7: copperleaf::PinRef = copperleaf::PinRef("7");
    pub const _PIN_8: copperleaf::PinRef = copperleaf::PinRef("8");
    pub const _PIN_9: copperleaf::PinRef = copperleaf::PinRef("9");
    pub const _PIN_10: copperleaf::PinRef = copperleaf::PinRef("10");
    pub const _PIN_11: copperleaf::PinRef = copperleaf::PinRef("11");
    pub const _PIN_12: copperleaf::PinRef = copperleaf::PinRef("12");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("~").number("1").pos(6.25, -7.75).rotation(0.0).length(2.5).width(1.75).height(2.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("~").number("3").pos(1.25, -7.75).rotation(0.0).length(2.5).width(1.75).height(2.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("~").number("5").pos(-3.75, -7.75).rotation(0.0).length(2.5).width(1.75).height(2.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("~").number("6").pos(-6.25, -7.75).rotation(0.0).length(2.5).width(1.75).height(2.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("~").number("7").pos(-6.25, 7.75).rotation(0.0).length(2.5).width(1.75).height(2.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("~").number("8").pos(-3.75, 7.75).rotation(0.0).length(2.5).width(1.75).height(2.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("~").number("9").pos(-1.25, 7.75).rotation(0.0).length(2.5).width(1.75).height(2.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("~").number("10").pos(1.25, 7.75).rotation(0.0).length(2.5).width(1.75).height(2.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("~").number("11").pos(3.75, 7.75).rotation(0.0).length(2.5).width(1.75).height(2.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("~").number("12").pos(6.25, 7.75).rotation(0.0).length(2.5).width(1.75).height(2.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
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

    pub fn layout_constraints(&self) -> Vec<copperleaf::LayoutConstraint> {
        use copperleaf::units::UnitExt;
        vec![
        ]
    }
}

impl copperleaf::Component for Comp749119250 {
    fn pins(&self) -> &[copperleaf::Pin] {
        &self.pins
    }

    fn constraints(&self) -> Vec<copperleaf::Constraint> {
        Self::constraints(self)
    }

    fn layout_constraints(&self) -> Vec<copperleaf::LayoutConstraint> {
        Self::layout_constraints(self)
    }

    fn mechanical(&self) -> &[copperleaf::Pad] {
        &self.mechanical
    }

    fn meta(&self) -> &copperleaf::ComponentMeta {
        static META: std::sync::OnceLock<copperleaf::ComponentMeta> = std::sync::OnceLock::new();
        META.get_or_init(|| copperleaf::ComponentMeta {
            symbol: Some("749119250".into()),
            
            footprint: Some("749119250".into()),
            
            datasheet: Some("https://www.we-online.com/components/products/datasheet/749119250.pdf".into()),
            
            
            description: None,
            model_3d: Some("749119250.step".into()),
            
            
            model_3d_data: None,
            
            model_3d_rotation: (0.0, 0.0, 0.0),
            
            model_3d_offset: (0.0, 0.0, 0.0),
            
            fab_extent: None,
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for Comp749119250 {
    fn default() -> Self {
        Self::new()
    }
}
