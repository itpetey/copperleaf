/// TDK B82472P6152M000 1.5uH SMT Power Inductor (7.3x7.3x4.5mm)
///
/// Datasheet: https://www.tdk-electronics.com/en/products/inductors/power-inductors/smt-power-inductors/b82472p6152m000
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | 1        | I/O         |                       |
/// | 2   | 2        | I/O         |                       |
pub struct B82472p6152m000 {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::MechanicalPad>,
}

impl B82472p6152m000 {
    pub const PIN_1: copperleaf::PinRef = copperleaf::PinRef("1");
    pub const PIN_2: copperleaf::PinRef = copperleaf::PinRef("2");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("1").number("1").pos(-3.2, 0.0).rotation(0.0).length(2.2).width(1.5).height(2.2).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("2").number("2").pos(3.2, 0.0).rotation(0.0).length(2.2).width(1.5).height(2.2).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
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

impl copperleaf::Component for B82472p6152m000 {
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
        Some("B82472P6152M000")
    }

    fn footprint(&self) -> Option<&'static str> {
        Some("B82472P6152M000")
    }

    fn datasheet(&self) -> Option<&'static str> {
        Some("https://www.tdk-electronics.com/en/products/inductors/power-inductors/smt-power-inductors/b82472p6152m000")
    }

    fn model_3d_data(&self) -> Option<&'static str> {
        Some("<elided:222024:8f70ae552d2524fe>")
    }

    fn model_3d_rotation(&self) -> (f64, f64, f64) {
        (-90.0, 0.0, 0.0)
    }
}

impl Default for B82472p6152m000 {
    fn default() -> Self {
        Self::new()
    }
}
