/// Abracon ARJM11 RJ45 Ethernet Connector with Magnetics, 10/100/1000 Base-T, PoE
///
/// Datasheet: https://abracon.com/Magnetics/ARJM11.pdf
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | TD2+     | I/O         |                       |
/// | 2   | TD1+     | I/O         |                       |
/// | 3   | TD3-     | I/O         |                       |
/// | 4   | CT_1     | I/O         |                       |
/// | 5   | ~        | I/O         |                       |
/// | 6   | ~        | I/O         |                       |
/// | 7   | TD1-     | I/O         |                       |
/// | 8   | SHIELD   | I/O         |                       |
/// | 9   | SHIELD   | I/O         |                       |
/// | 10  | TD3+     | I/O         |                       |
/// | 11  | ~        | I/O         |                       |
/// | 12  | ~        | I/O         |                       |
/// | 13  | TD4+     | I/O         |                       |
/// | 14  | TD4-     | I/O         |                       |
/// | 15  | TD2-     | I/O         |                       |
/// | 16  | CT_2     | I/O         |                       |
pub struct Arjm11d7502AbEw2 {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::MechanicalPad>,
}

impl Arjm11d7502AbEw2 {
    pub const TD2_: copperleaf::PinRef = copperleaf::PinRef("TD2+");
    pub const TD1_: copperleaf::PinRef = copperleaf::PinRef("TD1+");
    pub const TD3_: copperleaf::PinRef = copperleaf::PinRef("TD3-");
    pub const CT_1: copperleaf::PinRef = copperleaf::PinRef("CT_1");
    pub const _PIN: copperleaf::PinRef = copperleaf::PinRef("~");
    pub const TD1__7: copperleaf::PinRef = copperleaf::PinRef("TD1-");
    pub const SHIELD: copperleaf::PinRef = copperleaf::PinRef("SHIELD");
    pub const TD3__10: copperleaf::PinRef = copperleaf::PinRef("TD3+");
    pub const TD4_: copperleaf::PinRef = copperleaf::PinRef("TD4+");
    pub const TD4__14: copperleaf::PinRef = copperleaf::PinRef("TD4-");
    pub const TD2__15: copperleaf::PinRef = copperleaf::PinRef("TD2-");
    pub const CT_2: copperleaf::PinRef = copperleaf::PinRef("CT_2");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("TD2+").number("P3").pos(3.175, -6.35).rotation(0.0).length(1.458).width(1.458).height(1.458).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(0.95).dio(),
                Pin::build("TD1+").number("P1").pos(5.715, -6.35).rotation(0.0).length(1.458).width(1.458).height(1.458).pad_type("thru_hole").pad_shape("rect").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(0.95).dio(),
                Pin::build("TD3-").number("P8").pos(-3.175, -8.89).rotation(0.0).length(1.458).width(1.458).height(1.458).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(0.95).dio(),
                Pin::build("CT_1").number("P5").pos(0.635, -6.35).rotation(0.0).length(1.458).width(1.458).height(1.458).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(0.95).dio(),
                Pin::build("~").number("P12").pos(-6.785, 4.57).rotation(0.0).length(1.605).width(1.605).height(1.605).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(1.07).dio(),
                Pin::build("~").number("P11").pos(-6.785, 2.54).rotation(0.0).length(1.605).width(1.605).height(1.605).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(1.07).dio(),
                Pin::build("TD1-").number("P2").pos(4.445, -8.89).rotation(0.0).length(1.458).width(1.458).height(1.458).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(0.95).dio(),
                Pin::build("SHIELD").number("S1").pos(7.75, -3.05).rotation(0.0).length(2.475).width(2.475).height(2.475).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(1.65).dio(),
                Pin::build("SHIELD").number("S2").pos(-7.75, -3.05).rotation(0.0).length(2.475).width(2.475).height(2.475).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(1.65).dio(),
                Pin::build("TD3+").number("P7").pos(-1.905, -6.35).rotation(0.0).length(1.458).width(1.458).height(1.458).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(0.95).dio(),
                Pin::build("~").number("P14").pos(6.785, 4.57).rotation(0.0).length(1.605).width(1.605).height(1.605).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(1.07).dio(),
                Pin::build("~").number("P13").pos(6.785, 2.54).rotation(0.0).length(1.605).width(1.605).height(1.605).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(1.07).dio(),
                Pin::build("TD4+").number("P9").pos(-4.445, -6.35).rotation(0.0).length(1.458).width(1.458).height(1.458).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(0.95).dio(),
                Pin::build("TD4-").number("P10").pos(-5.715, -8.89).rotation(0.0).length(1.458).width(1.458).height(1.458).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(0.95).dio(),
                Pin::build("TD2-").number("P4").pos(1.905, -8.89).rotation(0.0).length(1.458).width(1.458).height(1.458).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(0.95).dio(),
                Pin::build("CT_2").number("P6").pos(-0.635, -8.89).rotation(0.0).length(1.458).width(1.458).height(1.458).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(0.95).dio(),
            ],
            mechanical: vec![
                copperleaf::MechanicalPad { number: "None".into(), pos: (5.715, 0.0), width: 3.3, height: 3.3, pad_type: "np_thru_hole".into(), pad_shape: "circle".into(), roundrect_rratio: None, layers: Some("*.Cu *.Mask".into()), drill: 3.3 },
                copperleaf::MechanicalPad { number: "None".into(), pos: (-5.715, 0.0), width: 3.3, height: 3.3, pad_type: "np_thru_hole".into(), pad_shape: "circle".into(), roundrect_rratio: None, layers: Some("*.Cu *.Mask".into()), drill: 3.3 },
            ],
        }
    }

    pub fn constraints(&self) -> Vec<copperleaf::Constraint> {
        use copperleaf::{Constraint, units::UnitExt};
        vec![
        ]
    }
}

impl copperleaf::Component for Arjm11d7502AbEw2 {
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
        Some("ARJM11D7-502-AB-EW2")
    }

    fn footprint(&self) -> Option<&'static str> {
        Some("ARJM11D7-502-AB-EW2")
    }

    fn datasheet(&self) -> Option<&'static str> {
        Some("https://abracon.com/Magnetics/ARJM11.pdf")
    }

    fn model_3d_data(&self) -> Option<&'static str> {
        Some("<elided:12565964:0a68908b10f63fd2>")
    }
}

impl Default for Arjm11d7502AbEw2 {
    fn default() -> Self {
        Self::new()
    }
}
