/// Texas Instruments TXU0304DTR 4-bit fixed-direction dual-supply voltage level translator (X2SON 12)
///
/// Datasheet: https://www.ti.com/lit/ds/symlink/txu0304.pdf
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | VCCA     | Supply      | A-port supply voltage. 1.1 V to 5.5 V. |
/// | 2   | A1       | Input       | Input A1. Referenced to VCCA. |
/// | 3   | A2       | Input       | Input A2. Referenced to VCCA. |
/// | 4   | A3       | Input       | Input A3. Referenced to VCCA. |
/// | 5   | A4Y      | Output      | Output A4. Referenced to VCCA. |
/// | 6   | GND      | Ground      |                       |
/// | 7   | B4       | Input       | Input B4. Referenced to VCCB. |
/// | 8   | B3Y      | Output      | Output B3. Referenced to VCCB. |
/// | 9   | B2Y      | Output      | Output B2. Referenced to VCCB. |
/// | 10  | B1Y      | Output      | Output B1. Referenced to VCCB. |
/// | 11  | VCCB     | Supply      | B-port supply voltage. 1.1 V to 5.5 V. |
/// | 12  | OE       | Input       | Output Enable. Pull to GND to place all outputs in high-impedance mode; pull to VCCA or VCCB to enable all outputs. |
pub struct Txu0304dtr {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Txu0304dtr {
    pub const VCCA: copperleaf::PinRef = copperleaf::PinRef("VCCA");
    pub const A1: copperleaf::PinRef = copperleaf::PinRef("A1");
    pub const A2: copperleaf::PinRef = copperleaf::PinRef("A2");
    pub const A3: copperleaf::PinRef = copperleaf::PinRef("A3");
    pub const A4Y: copperleaf::PinRef = copperleaf::PinRef("A4Y");
    pub const GND: copperleaf::PinRef = copperleaf::PinRef("GND");
    pub const B4: copperleaf::PinRef = copperleaf::PinRef("B4");
    pub const B3Y: copperleaf::PinRef = copperleaf::PinRef("B3Y");
    pub const B2Y: copperleaf::PinRef = copperleaf::PinRef("B2Y");
    pub const B1Y: copperleaf::PinRef = copperleaf::PinRef("B1Y");
    pub const VCCB: copperleaf::PinRef = copperleaf::PinRef("VCCB");
    pub const OE: copperleaf::PinRef = copperleaf::PinRef("OE");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("VCCA").number("1").pos(-0.515, -0.69).rotation(0.0).length(0.1).width(0.1).height(0.1).pad_type("smd").pad_shape("custom").layers("F.Cu").pwr(1.08.volt(), 5.5.volt(), 100.0.amp()).pin(),
                Pin::build("A1").number("2").pos(-0.22, -0.37).rotation(0.0).length(0.1).width(0.1).height(0.1).pad_type("smd").pad_shape("custom").layers("F.Cu").dio(),
                Pin::build("A2").number("3").pos(-0.5, 0.0).rotation(0.0).length(0.1).width(0.1).height(0.1).pad_type("smd").pad_shape("custom").layers("F.Cu").dio(),
                Pin::build("A3").number("4").pos(-0.22, 0.37).rotation(0.0).length(0.1).width(0.1).height(0.1).pad_type("smd").pad_shape("custom").layers("F.Cu").dio(),
                Pin::build("A4Y").number("5").pos(-0.515, 0.69).rotation(0.0).length(0.1).width(0.1).height(0.1).pad_type("smd").pad_shape("custom").layers("F.Cu").dio(),
                Pin::build("GND").number("6").pos(0.0, 0.85).rotation(0.0).length(0.1).width(0.1).height(0.1).pad_type("smd").pad_shape("custom").layers("F.Cu").gnd(),
                Pin::build("B4").number("7").pos(0.515, 0.69).rotation(0.0).length(0.1).width(0.1).height(0.1).pad_type("smd").pad_shape("custom").layers("F.Cu").dio(),
                Pin::build("B3Y").number("8").pos(0.22, 0.37).rotation(0.0).length(0.1).width(0.1).height(0.1).pad_type("smd").pad_shape("custom").layers("F.Cu").dio(),
                Pin::build("B2Y").number("9").pos(0.5, 0.0).rotation(0.0).length(0.1).width(0.1).height(0.1).pad_type("smd").pad_shape("custom").layers("F.Cu").dio(),
                Pin::build("B1Y").number("10").pos(0.22, -0.37).rotation(0.0).length(0.1).width(0.1).height(0.1).pad_type("smd").pad_shape("custom").layers("F.Cu").dio(),
                Pin::build("VCCB").number("11").pos(0.515, -0.69).rotation(0.0).length(0.1).width(0.1).height(0.1).pad_type("smd").pad_shape("custom").layers("F.Cu").pwr(1.08.volt(), 5.5.volt(), 100.0.amp()).pin(),
                Pin::build("OE").number("12").pos(0.0, -0.85).rotation(0.0).length(0.1).width(0.1).height(0.1).pad_type("smd").pad_shape("custom").layers("F.Cu").dio(),
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

impl copperleaf::Component for Txu0304dtr {
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
            symbol: Some("TXU0304DTR".into()),
            
            footprint: Some("TXU0304DTR".into()),
            
            datasheet: Some("https://www.ti.com/lit/ds/symlink/txu0304.pdf".into()),
            
            
            description: None,
            model_3d: Some("/Users/pete/Downloads/TXU0304DTR/TXU0304DTR.step".into()),
            
            model_3d_data: Some("<elided:794240:98068830d8f0ad96>".into()),
            
            
            model_3d_rotation: (0.0, 0.0, 0.0),
            
            model_3d_offset: (0.0, 0.0, 0.0),
            
            fab_extent: None,
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for Txu0304dtr {
    fn default() -> Self {
        Self::new()
    }
}
