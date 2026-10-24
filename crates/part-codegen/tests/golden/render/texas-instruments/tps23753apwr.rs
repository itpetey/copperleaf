/// Texas Instruments TPS23753APWR IEEE 802.3at PoE Interface and Converter Controller with Enhanced ESD Immunity (TSSOP-14)
///
/// Datasheet: https://www.ti.com/lit/ds/symlink/tps23753a.pdf
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | CTL      | Control input | Control loop input to the PWM. Use VB as a pull up for CTL; pulling VCTL below VZDC (~1.5 V) stops GATE switching. Input voltage range to RTN -0.3 to 6.5 V; input resistance 70-145 kOhm. |
/// | 2   | VB       | Bias rail   | Internal 5 V bias rail for the dc/dc control circuits; bypass to RTN with a 0.1 uF capacitor. May be used to bias an external feedback optocoupler. Sourcing current 0-5 mA. |
/// | 3   | CS       | Current sense | Dc/dc converter switching MOSFET current-sense input; connect to the high side of RCS. Current-limit threshold VCSMAX 0.50-0.60 V; turn-off delay 25-60 ns. |
/// | 4   | VC       | Bias supply | Dc/dc converter bias voltage. The internal startup current source and the converter bias winding output power this pin; connect a 0.22 uF minimum ceramic capacitor to RTN. Input voltage range to RTN 0-18 V. |
/// | 5   | GATE     | Gate drive  | Gate drive output for the dc/dc converter switching MOSFET. Source current 0.30-0.60 A; sink current 0.50-1.1 A. |
/// | 6   | RTN      | Return      | Negative rail input to the dc/dc converter and output of the PoE hotswap. Treat as a local ground reference plane. RTN current 350 mA max for TJ <= 125 C. |
/// | 7   | VSS      | Ground      | Negative power rail derived from the PoE source and reference for the PoE interface circuits. Clamped to a diode drop above RTN by the hotswap switch. |
/// | 8   | VDD1     | Startup supply | Source of dc/dc converter startup current; connect to VDD for most applications. May be isolated by a diode from VDD to support PoE priority operation. Input voltage range 0-57 V. |
/// | 9   | VDD      | Supply      | Positive input power rail for the PoE interface circuit, derived from the PoE source. Bypass to VSS with a 0.1 uF (X7R, 10%) capacitor and protect with a transient suppressor (Zener) diode. Input voltage range 0-57 V. |
/// | 10  | DEN      | Detection   | Connect a 24.9 kOhm resistor from DEN to VDD to provide the PoE detection signature. Pulling DEN to VSS during powered operation turns off the internal hotswap MOSFET and class regulator. Hotswap disable threshold VPD_DIS 3-5 V. |
/// | 11  | CLS      | Classification | Connect a resistor from CLS to VSS to program the classification current. RCLS values 1270, 243, 137 and 90.9 Ohm select PD classes 0-3; the output is disabled when APD is above VAPDEN. |
/// | 12  | APD      | Adapter detect | Pull APD above VAPDEN (~1.5 V) to disable the internal PD hotswap switch, forcing power to come from an external adapter; connect to the adapter through a resistor divider. Tie APD to RTN when not used. |
/// | 13  | BLNK     | Blanking    | Connect to RTN for the internally set blanking period (35-75 ns) or through a resistor to RTN to program it; RBLNK(kOhm) = tBLNK(ns). |
/// | 14  | FRS      | Frequency set | Connect a resistor from FRS to RTN to program the converter switching frequency; RFRS(kOhm) = 15000 / fSW(kHz). May be synchronized to a higher-frequency external reference. |
pub struct Tps23753apwr {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Tps23753apwr {
    pub const CTL: copperleaf::PinRef = copperleaf::PinRef("1");
    pub const VB: copperleaf::PinRef = copperleaf::PinRef("2");
    pub const CS: copperleaf::PinRef = copperleaf::PinRef("3");
    pub const VC: copperleaf::PinRef = copperleaf::PinRef("4");
    pub const GATE: copperleaf::PinRef = copperleaf::PinRef("5");
    pub const RTN: copperleaf::PinRef = copperleaf::PinRef("6");
    pub const VSS: copperleaf::PinRef = copperleaf::PinRef("7");
    pub const VDD1: copperleaf::PinRef = copperleaf::PinRef("8");
    pub const VDD: copperleaf::PinRef = copperleaf::PinRef("9");
    pub const DEN: copperleaf::PinRef = copperleaf::PinRef("10");
    pub const CLS: copperleaf::PinRef = copperleaf::PinRef("11");
    pub const APD: copperleaf::PinRef = copperleaf::PinRef("12");
    pub const BLNK: copperleaf::PinRef = copperleaf::PinRef("13");
    pub const FRS: copperleaf::PinRef = copperleaf::PinRef("14");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("CTL").number("1").pos(-2.87, -1.95).rotation(0.0).length(1.57).width(1.57).height(0.41).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("VB").number("2").pos(-2.87, -1.3).rotation(0.0).length(1.57).width(1.57).height(0.41).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("CS").number("3").pos(-2.87, -0.65).rotation(0.0).length(1.57).width(1.57).height(0.41).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("VC").number("4").pos(-2.87, 0.0).rotation(0.0).length(1.57).width(1.57).height(0.41).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GATE").number("5").pos(-2.87, 0.65).rotation(0.0).length(1.57).width(1.57).height(0.41).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("RTN").number("6").pos(-2.87, 1.3).rotation(0.0).length(1.57).width(1.57).height(0.41).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("VSS").number("7").pos(-2.87, 1.95).rotation(0.0).length(1.57).width(1.57).height(0.41).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("VDD1").number("8").pos(2.87, 1.95).rotation(0.0).length(1.57).width(1.57).height(0.41).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(0.0.volt(), 57.0.volt(), 0.35.amp()).pin(),
                Pin::build("VDD").number("9").pos(2.87, 1.3).rotation(0.0).length(1.57).width(1.57).height(0.41).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(0.0.volt(), 57.0.volt(), 0.35.amp()).pin(),
                Pin::build("DEN").number("10").pos(2.87, 0.65).rotation(0.0).length(1.57).width(1.57).height(0.41).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("CLS").number("11").pos(2.87, 0.0).rotation(0.0).length(1.57).width(1.57).height(0.41).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("APD").number("12").pos(2.87, -0.65).rotation(0.0).length(1.57).width(1.57).height(0.41).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("BLNK").number("13").pos(2.87, -1.3).rotation(0.0).length(1.57).width(1.57).height(0.41).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("FRS").number("14").pos(2.87, -1.95).rotation(0.0).length(1.57).width(1.57).height(0.41).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
            ],
            mechanical: vec![
            ],
        }
    }

    pub fn constraints(&self) -> Vec<copperleaf::Constraint> {
        use copperleaf::{Constraint, units::UnitExt};
        vec![
            Constraint::Decoupling { values: vec![100.0.nf(), 220.0.nf()], per_pin: true },
            Constraint::MaxJunction { temp: 125.0.celsius() },
        ]
    }

    pub fn layout_constraints(&self) -> Vec<copperleaf::LayoutConstraint> {
        use copperleaf::units::UnitExt;
        vec![
        ]
    }
}

impl copperleaf::Component for Tps23753apwr {
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
            symbol: Some("TPS23753APWR".into()),
            
            footprint: Some("TPS23753APWR".into()),
            
            datasheet: Some("https://www.ti.com/lit/ds/symlink/tps23753a.pdf".into()),
            
            
            description: None,
            model_3d: Some("TPS23753APWR.step".into()),
            
            model_3d_data: Some("<elided:686264:63415c5a31765b55>".into()),
            
            
            model_3d_rotation: (0.0, 0.0, 0.0),
            
            model_3d_offset: (0.0, 0.0, 0.0),
            
            fab_extent: None,
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for Tps23753apwr {
    fn default() -> Self {
        Self::new()
    }
}
