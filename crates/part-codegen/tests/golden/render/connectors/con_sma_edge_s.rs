/// PCB Mounting edge Connector SMA Female
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | Signal   | I/O         |                       |
/// | 1   | GND1     | I/O         |                       |
/// | 2   | GND2     | I/O         |                       |
/// | 3   | GND3     | I/O         |                       |
/// | 4   | GND4     | I/O         |                       |
pub struct ConSmaEdgeS {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl ConSmaEdgeS {
    pub const Signal: copperleaf::PinRef = copperleaf::PinRef("Signal");
    pub const GND1: copperleaf::PinRef = copperleaf::PinRef("GND1");
    pub const GND2: copperleaf::PinRef = copperleaf::PinRef("GND2");
    pub const GND3: copperleaf::PinRef = copperleaf::PinRef("GND3");
    pub const GND4: copperleaf::PinRef = copperleaf::PinRef("GND4");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("Signal").number("1").pos(0.0, 0.0).rotation(0.0).length(3.5).width(3.5).height(1.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GND1").number("G1").pos(0.0, -2.7).rotation(0.0).length(3.5).width(3.5).height(1.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GND2").number("G2").pos(0.0, 2.7).rotation(0.0).length(3.5).width(3.5).height(1.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GND3").number("G3").pos(0.0, -2.7).rotation(0.0).length(3.5).width(3.5).height(1.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("B.Cu B.Mask B.Paste").dio(),
                Pin::build("GND4").number("G4").pos(0.0, 2.7).rotation(0.0).length(3.5).width(3.5).height(1.5).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("B.Cu B.Mask B.Paste").dio(),
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

impl copperleaf::Component for ConSmaEdgeS {
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
            symbol: Some("CON-SMA-EDGE-S".into()),
            
            footprint: Some("CON-SMA-EDGE-S".into()),
            
            
            datasheet: None,
            
            description: None,
            
            model_3d: None,
            model_3d_data: Some("<elided:478480:3e4005f89e9ef6f3>".into()),
            
            model_3d_rotation: (-90.0, 0.0, 0.0),
            
            
            model_3d_offset: (0.0, 0.0, 0.0),
            
            fab_extent: None,
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for ConSmaEdgeS {
    fn default() -> Self {
        Self::new()
    }
}
