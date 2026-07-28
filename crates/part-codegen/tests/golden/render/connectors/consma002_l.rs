/// CONSMA002-L — SMA Female Extended Right-Angle Through-Hole Mount Connector
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | Signal   | RF Signal   |                       |
/// | 1   | GND1     | Ground      |                       |
/// | 2   | GND2     | Ground      |                       |
/// | 3   | GND3     | Ground      |                       |
/// | 4   | GND4     | Ground      |                       |
pub struct Consma002L {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Consma002L {
    pub const Signal: copperleaf::PinRef = copperleaf::PinRef("Signal");
    pub const GND1: copperleaf::PinRef = copperleaf::PinRef("GND1");
    pub const GND2: copperleaf::PinRef = copperleaf::PinRef("GND2");
    pub const GND3: copperleaf::PinRef = copperleaf::PinRef("GND3");
    pub const GND4: copperleaf::PinRef = copperleaf::PinRef("GND4");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("Signal").number("1").pos(0.0, 0.0).rotation(0.0).length(2.1).width(2.1).height(2.1).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(1.4).role(Role::AnalogIn).rf_limits().pin(),
                Pin::build("GND1").number("S1").pos(-2.55, -2.55).rotation(0.0).length(2.25).width(2.25).height(2.25).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(1.5).gnd(),
                Pin::build("GND2").number("S2").pos(-2.55, 2.55).rotation(0.0).length(2.25).width(2.25).height(2.25).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(1.5).gnd(),
                Pin::build("GND3").number("S3").pos(2.55, 2.55).rotation(0.0).length(2.25).width(2.25).height(2.25).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(1.5).gnd(),
                Pin::build("GND4").number("S4").pos(2.55, -2.55).rotation(0.0).length(2.25).width(2.25).height(2.25).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(1.5).gnd(),
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

impl copperleaf::Component for Consma002L {
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
            symbol: Some("CONSMA002-L".into()),
            
            footprint: Some("CONSMA002-L".into()),
            
            
            datasheet: None,
            
            description: None,
            
            model_3d: None,
            model_3d_data: Some("<elided:4470460:054bb2dd2cb2bb45>".into()),
            
            model_3d_rotation: (-90.0, 0.0, 0.0),
            
            model_3d_offset: (0.0, -6.25, 0.0),
            
            fab_extent: Some((-3.5, -3.5, 3.5, 16.0)),
            
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for Consma002L {
    fn default() -> Self {
        Self::new()
    }
}
