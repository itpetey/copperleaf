/// Texas Instruments TLC5946PWP 16-Channel, 12-Bit PWM LED Driver with 6-Bit Dot Correction (HTSSOP-28 PowerPAD)
///
/// Datasheet: https://www.ti.com/lit/ds/symlink/tlc5946.pdf
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | GND      | Ground      |                       |
/// | 2   | BLANK    | Input       | Blanking input. When high, all constant current outputs (OUT0-15) are forced off and the grayscale PWM counter resets to 0. |
/// | 3   | XLAT     | Input       | Edge-triggered latch input. On the rising edge, data is latched from the shift register into the dot correction register (MODE high) or grayscale register (MODE low). |
/// | 4   | SCLK     | Clock       | Serial data shift clock (up to 30 MHz). Data on SIN is clocked in on the rising edge. |
/// | 5   | SIN      | Input       | Serial data input, shifted in MSB-first on each rising edge of SCLK. |
/// | 6   | MODE     | Input       | Input mode select. High selects dot correction (DC, 96-bit shift register); low selects grayscale PWM (GS, 192-bit shift register). |
/// | 7   | OUT0     | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 8   | OUT1     | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 9   | OUT2     | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 10  | OUT3     | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 11  | OUT4     | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 12  | OUT5     | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 13  | OUT6     | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 14  | OUT7     | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 15  | OUT8     | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 16  | OUT9     | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 17  | OUT10    | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 18  | OUT11    | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 19  | OUT12    | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 20  | OUT13    | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 21  | OUT14    | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 22  | OUT15    | LED output  | Constant current sink output, 4 mA to 40 mA, up to 17 V. |
/// | 23  | XERR     | Output      | Open-drain error output. Goes low when an LED open (LOD) or thermal error (TEF) is detected. |
/// | 24  | SOUT     | Output      | Serial data output for cascading devices; also shifts out status information data (SID). |
/// | 25  | GSCLK    | Clock       | Grayscale PWM reference clock (up to 33 MHz). |
/// | 26  | XHALF    | Input       | Extended serial interface select. High: normal operation. Low: extended serial interface active. |
/// | 27  | IREF     | Reference   | Reference current set input. Connect an external resistor (1.275 kOhm to 12.75 kOhm) between IREF and GND to set the maximum output current (4 mA to 40 mA). |
/// | 28  | VCC      | Supply      | Power-supply input, 3.0 V to 5.5 V. Supply current up to 45 mA. |
/// | 29  | EPAD     | Thermal Pad | Exposed PowerPAD thermal pad. Must be soldered to the PCB copper area for thermal performance. |
pub struct Tlc5946pwp {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Tlc5946pwp {
    pub const GND: copperleaf::PinRef = copperleaf::PinRef("1");
    pub const BLANK: copperleaf::PinRef = copperleaf::PinRef("2");
    pub const XLAT: copperleaf::PinRef = copperleaf::PinRef("3");
    pub const SCLK: copperleaf::PinRef = copperleaf::PinRef("4");
    pub const SIN: copperleaf::PinRef = copperleaf::PinRef("5");
    pub const MODE: copperleaf::PinRef = copperleaf::PinRef("6");
    pub const OUT0: copperleaf::PinRef = copperleaf::PinRef("7");
    pub const OUT1: copperleaf::PinRef = copperleaf::PinRef("8");
    pub const OUT2: copperleaf::PinRef = copperleaf::PinRef("9");
    pub const OUT3: copperleaf::PinRef = copperleaf::PinRef("10");
    pub const OUT4: copperleaf::PinRef = copperleaf::PinRef("11");
    pub const OUT5: copperleaf::PinRef = copperleaf::PinRef("12");
    pub const OUT6: copperleaf::PinRef = copperleaf::PinRef("13");
    pub const OUT7: copperleaf::PinRef = copperleaf::PinRef("14");
    pub const OUT8: copperleaf::PinRef = copperleaf::PinRef("15");
    pub const OUT9: copperleaf::PinRef = copperleaf::PinRef("16");
    pub const OUT10: copperleaf::PinRef = copperleaf::PinRef("17");
    pub const OUT11: copperleaf::PinRef = copperleaf::PinRef("18");
    pub const OUT12: copperleaf::PinRef = copperleaf::PinRef("19");
    pub const OUT13: copperleaf::PinRef = copperleaf::PinRef("20");
    pub const OUT14: copperleaf::PinRef = copperleaf::PinRef("21");
    pub const OUT15: copperleaf::PinRef = copperleaf::PinRef("22");
    pub const XERR: copperleaf::PinRef = copperleaf::PinRef("23");
    pub const SOUT: copperleaf::PinRef = copperleaf::PinRef("24");
    pub const GSCLK: copperleaf::PinRef = copperleaf::PinRef("25");
    pub const XHALF: copperleaf::PinRef = copperleaf::PinRef("26");
    pub const IREF: copperleaf::PinRef = copperleaf::PinRef("27");
    pub const VCC: copperleaf::PinRef = copperleaf::PinRef("28");
    pub const EPAD: copperleaf::PinRef = copperleaf::PinRef("29");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("GND").number("1").pos(-2.8, -4.225).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").gnd(),
                Pin::build("BLANK").number("2").pos(-2.8, -3.575).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("XLAT").number("3").pos(-2.8, -2.925).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("SCLK").number("4").pos(-2.8, -2.275).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").clk(30.0),
                Pin::build("SIN").number("5").pos(-2.8, -1.625).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("MODE").number("6").pos(-2.8, -0.975).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT0").number("7").pos(-2.8, -0.325).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT1").number("8").pos(-2.8, 0.325).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT2").number("9").pos(-2.8, 0.975).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT3").number("10").pos(-2.8, 1.625).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT4").number("11").pos(-2.8, 2.275).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT5").number("12").pos(-2.8, 2.925).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT6").number("13").pos(-2.8, 3.575).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT7").number("14").pos(-2.8, 4.225).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT8").number("15").pos(2.8, 4.225).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT9").number("16").pos(2.8, 3.575).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT10").number("17").pos(2.8, 2.925).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT11").number("18").pos(2.8, 2.275).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT12").number("19").pos(2.8, 1.625).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT13").number("20").pos(2.8, 0.975).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT14").number("21").pos(2.8, 0.325).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("OUT15").number("22").pos(2.8, -0.325).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("XERR").number("23").pos(2.8, -0.975).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("SOUT").number("24").pos(2.8, -1.625).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("GSCLK").number("25").pos(2.8, -2.275).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").clk(33.0),
                Pin::build("XHALF").number("26").pos(2.8, -2.925).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("IREF").number("27").pos(2.8, -3.575).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").dio(),
                Pin::build("VCC").number("28").pos(2.8, -4.225).rotation(0.0).length(1.6).width(1.6).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu").pwr(3.0.volt(), 5.5.volt(), 0.045.amp()).pin(),
                Pin::build("EPAD").number("29").pos(0.0, 0.0).rotation(0.0).length(9.7).width(3.4).height(9.7).pad_type("smd").pad_shape("rect").layers("F.Cu").gnd(),
            ],
            mechanical: vec![
            ],
        }
    }

    pub fn constraints(&self) -> Vec<copperleaf::Constraint> {
        use copperleaf::{Constraint, units::UnitExt};
        vec![
            Constraint::MaxJunction { temp: 125.0.celsius() },
        ]
    }

    pub fn layout_constraints(&self) -> Vec<copperleaf::LayoutConstraint> {
        use copperleaf::units::UnitExt;
        vec![
        ]
    }
}

impl copperleaf::Component for Tlc5946pwp {
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
            symbol: Some("TLC5946PWP".into()),
            
            footprint: Some("TLC5946PWP".into()),
            
            datasheet: Some("https://www.ti.com/lit/ds/symlink/tlc5946.pdf".into()),
            
            
            description: None,
            model_3d: Some("TLC5946PWP.step".into()),
            
            model_3d_data: Some("<elided:1715084:735e5bd57cd97828>".into()),
            
            
            model_3d_rotation: (0.0, 0.0, 0.0),
            
            model_3d_offset: (0.0, 0.0, 0.0),
            
            fab_extent: None,
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for Tlc5946pwp {
    fn default() -> Self {
        Self::new()
    }
}
