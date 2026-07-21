/// Microchip MCP73831 Single Cell Li-Ion/Li-Polymer Charge Controller (SOT-23-5)
///
/// Datasheet: https://ww1.microchip.com/downloads/aemDocuments/documents/APID/ProductDocuments/DataSheets/MCP73831-Family-Data-Sheet-DS20001984H.pdf
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | STAT     | I/O         |                       |
/// | 2   | VSS      | Ground      |                       |
/// | 3   | VBAT     | Battery output |                       |
/// | 4   | VDD      | Supply      |                       |
/// | 5   | PROG     | I/O         |                       |
pub struct Mcp73831t2atiOt {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::MechanicalPad>,
}

impl Mcp73831t2atiOt {
    pub const STAT: copperleaf::PinRef = copperleaf::PinRef("STAT");
    pub const VSS: copperleaf::PinRef = copperleaf::PinRef("VSS");
    pub const VBAT: copperleaf::PinRef = copperleaf::PinRef("VBAT");
    pub const VDD: copperleaf::PinRef = copperleaf::PinRef("VDD");
    pub const PROG: copperleaf::PinRef = copperleaf::PinRef("PROG");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("STAT").number("1").pos(-1.245, -0.95).rotation(0.0).length(1.22).width(1.22).height(0.6).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("VSS").number("2").pos(-1.245, 0.0).rotation(0.0).length(1.22).width(1.22).height(0.6).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("VBAT").number("3").pos(-1.245, 0.95).rotation(0.0).length(1.22).width(1.22).height(0.6).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").role(Role::PowerOut).power_spec(PowerSpec { v_min: 4.2.volt(), v_max: 4.2.volt(), v_nom: Some(4.2.volt()), i_max: 0.5.amp() }).pin(),
                Pin::build("VDD").number("4").pos(1.245, 0.95).rotation(0.0).length(1.22).width(1.22).height(0.6).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(4.5.volt(), 6.0.volt(), 1.0.amp()).pin(),
                Pin::build("PROG").number("5").pos(1.245, -0.95).rotation(0.0).length(1.22).width(1.22).height(0.6).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
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

impl copperleaf::Component for Mcp73831t2atiOt {
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
        Some("MCP73831T-2ATI_OT")
    }

    fn footprint(&self) -> Option<&'static str> {
        Some("MCP73831T-2ATI_OT")
    }

    fn datasheet(&self) -> Option<&'static str> {
        Some("https://ww1.microchip.com/downloads/aemDocuments/documents/APID/ProductDocuments/DataSheets/MCP73831-Family-Data-Sheet-DS20001984H.pdf")
    }

    fn model_3d_data(&self) -> Option<&'static str> {
        Some("<elided:252876:0aacf1d6589cc2f1>")
    }

    fn model_3d_rotation(&self) -> (f64, f64, f64) {
        (-90.0, 0.0, 0.0)
    }
}

impl Default for Mcp73831t2atiOt {
    fn default() -> Self {
        Self::new()
    }
}
