/// Battery Holder (Open) CR123A 1 Cell PC Pin
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | POSITIVE | I/O         |                       |
/// | 2   | NEGATIVE | I/O         |                       |
pub struct Bh123a {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Bh123a {
    pub const POSITIVE: copperleaf::PinRef = copperleaf::PinRef("POSITIVE");
    pub const NEGATIVE: copperleaf::PinRef = copperleaf::PinRef("NEGATIVE");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("POSITIVE").number("P").pos(18.985, 0.0).rotation(0.0).length(2.205).width(2.205).height(2.205).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(1.47).dio(),
                Pin::build("NEGATIVE").number("N").pos(-18.985, 0.0).rotation(0.0).length(2.205).width(2.205).height(2.205).pad_type("thru_hole").pad_shape("circle").solder_mask_margin(0.102).layers("*.Cu *.Mask").drill(1.47).dio(),
            ],
            mechanical: vec![
                copperleaf::Pad { number: String::new(), pos: (19.58, 6.37), rotation: 0.0, width: 1.27, height: 1.27, pad_type: copperleaf::PadType::NpThruHole, pad_shape: copperleaf::PadShape::Circle, roundrect_rratio: None, solder_mask_margin: None, layers: Some("*.Cu *.Mask".into()), drill: Some(1.27) },
            ],
        }
    }

    pub fn constraints(&self) -> Vec<copperleaf::Constraint> {
        use copperleaf::{Constraint, units::UnitExt};
        vec![
        ]
    }
}

impl copperleaf::Component for Bh123a {
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
            symbol: Some("BH123A".into()),
            
            footprint: Some("BH123A".into()),
            
            
            datasheet: None,
            
            description: None,
            
            model_3d: None,
            model_3d_data: Some("<elided:778128:1c43e8a3136dc0d4>".into()),
            
            
            model_3d_rotation: (0.0, 0.0, 0.0),
            
            model_3d_offset: (0.0, 0.0, 0.0),
            fab_extent: Some((-21.5, -8.89, 21.5, 8.89)),
            
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for Bh123a {
    fn default() -> Self {
        Self::new()
    }
}
