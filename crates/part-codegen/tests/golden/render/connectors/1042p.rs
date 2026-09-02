/// 1042P — SMT Polarized Battery Holder for 18650 Battery | Keystone Electronics 1042P
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | POSITIVE | I/O         |                       |
/// | 2   | NEGATIVE | I/O         |                       |
pub struct Comp1042p {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Comp1042p {
    pub const POSITIVE: copperleaf::PinRef = copperleaf::PinRef("POSITIVE");
    pub const NEGATIVE: copperleaf::PinRef = copperleaf::PinRef("NEGATIVE");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("POSITIVE").number("P").pos(39.67, 0.0).rotation(0.0).length(7.46).width(7.46).height(6.47).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("NEGATIVE").number("N").pos(-39.67, 0.0).rotation(0.0).length(7.46).width(7.46).height(6.47).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
            ],
            mechanical: vec![
                copperleaf::Pad { number: String::new(), pos: (27.6, -8.0), rotation: 0.0, width: 3.45, height: 3.45, pad_type: copperleaf::PadType::NpThruHole, pad_shape: copperleaf::PadShape::Circle, roundrect_rratio: None, solder_mask_margin: None, layers: Some("*.Cu *.Mask".into()), drill: Some(3.45) },
                copperleaf::Pad { number: String::new(), pos: (-27.6, 8.0), rotation: 0.0, width: 3.45, height: 3.45, pad_type: copperleaf::PadType::NpThruHole, pad_shape: copperleaf::PadShape::Circle, roundrect_rratio: None, solder_mask_margin: None, layers: Some("*.Cu *.Mask".into()), drill: Some(3.45) },
                copperleaf::Pad { number: String::new(), pos: (35.82, 8.0), rotation: 0.0, width: 2.39, height: 2.39, pad_type: copperleaf::PadType::NpThruHole, pad_shape: copperleaf::PadShape::Circle, roundrect_rratio: None, solder_mask_margin: None, layers: Some("*.Cu *.Mask".into()), drill: Some(2.39) },
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

impl copperleaf::Component for Comp1042p {
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
            symbol: Some("1042P".into()),
            
            footprint: Some("1042P".into()),
            
            
            datasheet: None,
            
            description: None,
            
            model_3d: None,
            model_3d_data: Some("<elided:3834760:ae3ab6840a23e320>".into()),
            
            model_3d_rotation: (-90.0, 0.0, -90.0),
            
            
            model_3d_offset: (0.0, 0.0, 0.0),
            
            fab_extent: None,
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for Comp1042p {
    fn default() -> Self {
        Self::new()
    }
}
