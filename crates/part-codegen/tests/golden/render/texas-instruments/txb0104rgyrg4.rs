/// Texas Instruments TXB0104RGYRG4 4-Bit Bidirectional Voltage-Level Translator with Automatic Direction Sensing and ±15-kV ESD Protection (14-VQFN)
///
/// Datasheet: https://www.ti.com/lit/ds/symlink/txb0104.pdf
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | VCCA     | Supply      | A-port supply voltage. 1.2 V ≤ VCCA ≤ 3.6 V and VCCA ≤ VCCB. |
/// | 2   | A1       | I/O         | Input/output 1. Referenced to VCCA. |
/// | 3   | A2       | I/O         | Input/output 2. Referenced to VCCA. |
/// | 4   | A3       | I/O         | Input/output 3. Referenced to VCCA. |
/// | 5   | A4       | I/O         | Input/output 4. Referenced to VCCA. |
/// | 6   | NC_6     | I/O         | No connection. Not internally connected. |
/// | 7   | GND      | Ground      |                       |
/// | 8   | OE       | Input       | Tri-state output-mode enable. Pull OE low to place all outputs in tri-state mode. Referenced to VCCA. |
/// | 9   | NC_9     | I/O         | No connection. Not internally connected. |
/// | 10  | B4       | I/O         | Input/output 4. Referenced to VCCB. |
/// | 11  | B3       | I/O         | Input/output 3. Referenced to VCCB. |
/// | 12  | B2       | I/O         | Input/output 2. Referenced to VCCB. |
/// | 13  | B1       | I/O         | Input/output 1. Referenced to VCCB. |
/// | 14  | VCCB     | Supply      | B-port supply voltage. 1.65 V ≤ VCCB ≤ 5.5 V. |
/// | 15  | EXP      | Thermal Pad | Exposed center thermal pad. For the RGY package, must be connected to Ground or left electrically open. |
pub struct Txb0104rgyrg4 {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Txb0104rgyrg4 {
    pub const VCCA: copperleaf::PinRef = copperleaf::PinRef("VCCA");
    pub const A1: copperleaf::PinRef = copperleaf::PinRef("A1");
    pub const A2: copperleaf::PinRef = copperleaf::PinRef("A2");
    pub const A3: copperleaf::PinRef = copperleaf::PinRef("A3");
    pub const A4: copperleaf::PinRef = copperleaf::PinRef("A4");
    pub const NC_6: copperleaf::PinRef = copperleaf::PinRef("NC_6");
    pub const GND: copperleaf::PinRef = copperleaf::PinRef("GND");
    pub const OE: copperleaf::PinRef = copperleaf::PinRef("OE");
    pub const NC_9: copperleaf::PinRef = copperleaf::PinRef("NC_9");
    pub const B4: copperleaf::PinRef = copperleaf::PinRef("B4");
    pub const B3: copperleaf::PinRef = copperleaf::PinRef("B3");
    pub const B2: copperleaf::PinRef = copperleaf::PinRef("B2");
    pub const B1: copperleaf::PinRef = copperleaf::PinRef("B1");
    pub const VCCB: copperleaf::PinRef = copperleaf::PinRef("VCCB");
    pub const EXP: copperleaf::PinRef = copperleaf::PinRef("EXP");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("VCCA").number("1").pos(-1.685, 0.75).rotation(0.0).length(0.9).width(0.9).height(0.26).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(1.2.volt(), 3.6.volt(), 0.1.amp()).pin(),
                Pin::build("A1").number("2").pos(-1.0, 1.685).rotation(0.0).length(0.9).width(0.26).height(0.9).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("A2").number("3").pos(-0.5, 1.685).rotation(0.0).length(0.9).width(0.26).height(0.9).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("A3").number("4").pos(0.0, 1.685).rotation(0.0).length(0.9).width(0.26).height(0.9).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("A4").number("5").pos(0.5, 1.685).rotation(0.0).length(0.9).width(0.26).height(0.9).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("NC_6").number("6").pos(1.0, 1.685).rotation(0.0).length(0.9).width(0.26).height(0.9).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").nc(true).dio(),
                Pin::build("GND").number("7").pos(1.685, 0.75).rotation(0.0).length(0.9).width(0.9).height(0.26).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("OE").number("8").pos(1.685, -0.75).rotation(0.0).length(0.9).width(0.9).height(0.26).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("NC_9").number("9").pos(1.0, -1.685).rotation(0.0).length(0.9).width(0.26).height(0.9).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").nc(true).dio(),
                Pin::build("B4").number("10").pos(0.5, -1.685).rotation(0.0).length(0.9).width(0.26).height(0.9).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("B3").number("11").pos(0.0, -1.685).rotation(0.0).length(0.9).width(0.26).height(0.9).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("B2").number("12").pos(-0.5, -1.685).rotation(0.0).length(0.9).width(0.26).height(0.9).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("B1").number("13").pos(-1.0, -1.685).rotation(0.0).length(0.9).width(0.26).height(0.9).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("VCCB").number("14").pos(-1.685, -0.75).rotation(0.0).length(0.9).width(0.9).height(0.26).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(1.65.volt(), 5.5.volt(), 0.1.amp()).pin(),
                Pin::build("EXP").number("15").pos(0.0, 0.0).rotation(0.0).length(2.05).width(2.05).height(2.05).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask").gnd(),
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

impl copperleaf::Component for Txb0104rgyrg4 {
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
            symbol: Some("TXB0104RGYRG4".into()),
            
            footprint: Some("TXB0104RGYRG4".into()),
            
            datasheet: Some("https://www.ti.com/lit/ds/symlink/txb0104.pdf".into()),
            
            
            description: None,
            model_3d: Some("/Users/pete/Downloads/TXB0104RGYRG4/TXB0104RGYRG4.step".into()),
            
            model_3d_data: Some("<elided:578756:5096c791c6835f7a>".into()),
            
            model_3d_rotation: (-90.0, 0.0, -90.0),
            
            
            model_3d_offset: (0.0, 0.0, 0.0),
            
            fab_extent: None,
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for Txb0104rgyrg4 {
    fn default() -> Self {
        Self::new()
    }
}
