/// Texas Instruments TLV75518PDBVR 500-mA high-PSRR low-IQ low-dropout voltage regulator with enable (SOT-23-5)
///
/// Datasheet: https://www.ti.com/lit/ds/symlink/tlv755p.pdf
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | IN       | I/O         | Input pin. A 1 µF or larger capacitor is required from this pin to ground. |
/// | 2   | GND      | Ground      | Ground pin.           |
/// | 3   | EN       | I/O         | Enable pin. Drive EN above VHI (1 V) to turn on the regulator; drive below VLO (0.3 V) to place the LDO into shutdown mode. If shutdown capability is not required, connect EN to IN. |
/// | 4   | NC       | No connect  | No internal connection |
/// | 5   | OUT      | I/O         | Regulated output voltage pin (1.8 V). A 1 µF or larger capacitor is required from this pin to ground. |
pub struct Tlv75518pdbvr {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Tlv75518pdbvr {
    pub const IN: copperleaf::PinRef = copperleaf::PinRef("IN");
    pub const GND: copperleaf::PinRef = copperleaf::PinRef("GND");
    pub const EN: copperleaf::PinRef = copperleaf::PinRef("EN");
    pub const NC: copperleaf::PinRef = copperleaf::PinRef("NC");
    pub const OUT: copperleaf::PinRef = copperleaf::PinRef("OUT");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("IN").number("1").pos(-1.255, -0.95).rotation(0.0).length(1.21).width(1.21).height(0.59).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GND").number("2").pos(-1.255, 0.0).rotation(0.0).length(1.21).width(1.21).height(0.59).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("EN").number("3").pos(-1.255, 0.95).rotation(0.0).length(1.21).width(1.21).height(0.59).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("NC").number("4").pos(1.255, 0.95).rotation(0.0).length(1.21).width(1.21).height(0.59).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("OUT").number("5").pos(1.255, -0.95).rotation(0.0).length(1.21).width(1.21).height(0.59).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
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

impl copperleaf::Component for Tlv75518pdbvr {
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
            symbol: Some("TLV75518PDBVR".into()),
            
            footprint: Some("TLV75518PDBVR".into()),
            
            datasheet: Some("https://www.ti.com/lit/ds/symlink/tlv755p.pdf".into()),
            
            
            description: None,
            
            model_3d: None,
            model_3d_data: Some("<elided:354464:dac8b12d90a187ad>".into()),
            
            model_3d_rotation: (-90.0, 0.0, 0.0),
            
            
            model_3d_offset: (0.0, 0.0, 0.0),
            
            fab_extent: None,
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for Tlv75518pdbvr {
    fn default() -> Self {
        Self::new()
    }
}
