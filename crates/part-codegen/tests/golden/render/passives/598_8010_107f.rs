/// 598-8010-107F — LED; 0603 SMT LED,RED,ALINGAP,WATER CLEAR LENS,140 DEG VIEWING ANGLE | Dialight 598-8010-107F
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | C        | I/O         |                       |
/// | 2   | A        | I/O         |                       |
pub struct Comp5988010107f {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Comp5988010107f {
    pub const C: copperleaf::PinRef = copperleaf::PinRef("C");
    pub const A: copperleaf::PinRef = copperleaf::PinRef("A");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("C").number("C").pos(-0.75, 0.0).rotation(0.0).length(0.91).width(0.91).height(0.83).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.265).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("A").number("A").pos(0.75, 0.0).rotation(0.0).length(0.91).width(0.91).height(0.83).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.265).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
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

impl copperleaf::Component for Comp5988010107f {
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
            symbol: Some("598-8010-107F".into()),
            
            footprint: Some("598-8010-107F".into()),
            
            
            datasheet: None,
            
            description: None,
            
            model_3d: None,
            model_3d_data: Some("<elided:358420:6caf3295480ce4b0>".into()),
            
            model_3d_rotation: (-90.0, 0.0, 0.0),
            
            
            model_3d_offset: (0.0, 0.0, 0.0),
            
            fab_extent: None,
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for Comp5988010107f {
    fn default() -> Self {
        Self::new()
    }
}
