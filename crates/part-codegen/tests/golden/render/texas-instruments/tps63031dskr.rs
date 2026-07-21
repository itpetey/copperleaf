/// Texas Instruments TPS63031 Buck-Boost Converter with 1-A Switches (QFN-10)
///
/// Datasheet: https://www.ti.com/lit/gpn/TPS63031
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | VOUT     | I/O         |                       |
/// | 2   | L2       | I/O         |                       |
/// | 3   | PGND     | Ground      |                       |
/// | 4   | L1       | I/O         |                       |
/// | 5   | VIN      | I/O         |                       |
/// | 6   | EN       | I/O         |                       |
/// | 7   | PS/SYNC  | I/O         |                       |
/// | 8   | VINA     | I/O         |                       |
/// | 9   | GND      | Ground      |                       |
/// | 10  | FB       | I/O         |                       |
/// | 11  | EXP      | I/O         |                       |
pub struct Tps63031dskr {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Tps63031dskr {
    pub const VOUT: copperleaf::PinRef = copperleaf::PinRef("VOUT");
    pub const L2: copperleaf::PinRef = copperleaf::PinRef("L2");
    pub const PGND: copperleaf::PinRef = copperleaf::PinRef("PGND");
    pub const L1: copperleaf::PinRef = copperleaf::PinRef("L1");
    pub const VIN: copperleaf::PinRef = copperleaf::PinRef("VIN");
    pub const EN: copperleaf::PinRef = copperleaf::PinRef("EN");
    pub const PS_SYNC: copperleaf::PinRef = copperleaf::PinRef("PS/SYNC");
    pub const VINA: copperleaf::PinRef = copperleaf::PinRef("VINA");
    pub const GND: copperleaf::PinRef = copperleaf::PinRef("GND");
    pub const FB: copperleaf::PinRef = copperleaf::PinRef("FB");
    pub const EXP: copperleaf::PinRef = copperleaf::PinRef("EXP");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("VOUT").number("1").pos(-1.195, -1.0).rotation(0.0).length(0.84).width(0.84).height(0.27).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("L2").number("2").pos(-1.195, -0.5).rotation(0.0).length(0.84).width(0.84).height(0.27).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("PGND").number("3").pos(-1.195, 0.0).rotation(0.0).length(0.84).width(0.84).height(0.27).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("L1").number("4").pos(-1.195, 0.5).rotation(0.0).length(0.84).width(0.84).height(0.27).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("VIN").number("5").pos(-1.195, 1.0).rotation(0.0).length(0.84).width(0.84).height(0.27).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("EN").number("6").pos(1.195, 1.0).rotation(0.0).length(0.84).width(0.84).height(0.27).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("PS/SYNC").number("7").pos(1.195, 0.5).rotation(0.0).length(0.84).width(0.84).height(0.27).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("VINA").number("8").pos(1.195, 0.0).rotation(0.0).length(0.84).width(0.84).height(0.27).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GND").number("9").pos(1.195, -0.5).rotation(0.0).length(0.84).width(0.84).height(0.27).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("FB").number("10").pos(1.195, -1.0).rotation(0.0).length(0.84).width(0.84).height(0.27).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("EXP").number("11").pos(0.0, 0.0).rotation(0.0).length(2.0).width(1.2).height(2.0).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask").thermal_via((0.35, 0.0), 0.2, 0.3).thermal_via((-0.35, 0.0), 0.2, 0.3).thermal_via((0.0, -0.75), 0.2, 0.3).thermal_via((0.0, 0.75), 0.2, 0.3).dio(),
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

impl copperleaf::Component for Tps63031dskr {
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
            symbol: Some("TPS63031DSKR".into()),
            
            footprint: Some("TPS63031DSKR".into()),
            
            datasheet: Some("https://www.ti.com/lit/gpn/TPS63031".into()),
            
            
            description: None,
            
            model_3d: None,
            model_3d_data: Some("<elided:395136:663053e6dfee47fa>".into()),
            
            model_3d_rotation: (-90.0, 0.0, 0.0),
            
            
            model_3d_offset: (0.0, 0.0, 0.0),
        })
    }
}

impl Default for Tps63031dskr {
    fn default() -> Self {
        Self::new()
    }
}
