/// WIZnet W5500 Hardwired TCP/IP Ethernet controller with integrated 10/100 PHY (48-LQFP).
///
/// WIZnet W5500 Ethernet controller
///
/// Datasheet: https://docs.wiznet.io/img/products/w5500/W5500_ds_v110e.pdf
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | TXN      | Ethernet PHY | Differential transmit (negative) |
/// | 2   | TXP      | Ethernet PHY | Differential transmit (positive) |
/// | 3   | AGND     | Analog ground |                       |
/// | 4   | AVDD     | Analog supply | Analog 3.3V power     |
/// | 5   | RXN      | Ethernet PHY | Differential receive (negative) |
/// | 6   | RXP      | Ethernet PHY | Differential receive (positive) |
/// | 7   | DNC      | Do not connect | Do not connect        |
/// | 8   | AVDD     | Analog supply | Analog 3.3V power     |
/// | 9   | AGND     | Analog ground |                       |
/// | 10  | EXRES1   | PHY bias    | External reference resistor; 12.4kΩ 1% to AGND |
/// | 11  | AVDD     | Analog supply | Analog 3.3V power     |
/// | 12  | NC       | No connect  | Do not connect        |
/// | 13  | NC       | No connect  | Do not connect        |
/// | 14  | AGND     | Analog ground |                       |
/// | 15  | AVDD     | Analog supply | Analog 3.3V power     |
/// | 16  | AGND     | Analog ground |                       |
/// | 17  | AVDD     | Analog supply | Analog 3.3V power     |
/// | 18  | VBG      | Band gap reference | Band gap output, ~1.2V at 25°C; leave floating |
/// | 19  | AGND     | Analog ground |                       |
/// | 20  | TOCAP    | Reference capacitor | External reference capacitor; 4.7µF to AGND, keep trace short |
/// | 21  | AVDD     | Analog supply | Analog 3.3V power     |
/// | 22  | 1V2O     | Regulator output | 1.2V regulator output; 10nF to GND |
/// | 23  | RSVD     | Reserved    | Must be tied to GND   |
/// | 24  | SPDLED   | Speed LED   | Low: 100Mbps, High: 10Mbps |
/// | 25  | LINKLED  | Link LED    | Low: link established, High: no link |
/// | 26  | DUPLED   | Duplex LED  | Low: full-duplex, High: half-duplex |
/// | 27  | ACTLED   | Active LED  | Low: carrier sense during TX/RX activity |
/// | 28  | VDD      | Digital supply | Digital 3.3V power    |
/// | 29  | GND      | Digital ground |                       |
/// | 30  | XI/CLKIN | Crystal     | 25MHz crystal input or external 3.3V clock input |
/// | 31  | XO       | Crystal     | 25MHz crystal output; float when using external clock on XI/CLKIN |
/// | 32  | SCSn     | SPI         | Active-low SPI chip select (internal pull-up) |
/// | 33  | SCLK     | SPI         | SPI clock input       |
/// | 34  | MISO     | SPI         | SPI master in / slave out; high-Z when SCSn high |
/// | 35  | MOSI     | SPI         | SPI master out / slave in |
/// | 36  | INTn     | Interrupt   | Active-low interrupt output |
/// | 37  | RSTn     | Reset       | Active-low reset, hold low ≥500µs (internal pull-up) |
/// | 38  | RSVD     | Reserved    | NC, internal pull-down |
/// | 39  | RSVD     | Reserved    | NC, internal pull-down |
/// | 40  | RSVD     | Reserved    | NC, internal pull-down |
/// | 41  | RSVD     | Reserved    | NC, internal pull-down |
/// | 42  | RSVD     | Reserved    | NC, internal pull-down |
/// | 43  | PMODE2   | PHY mode    | PHY operation mode select bit 2 (pull-up) |
/// | 44  | PMODE1   | PHY mode    | PHY operation mode select bit 1 (pull-up) |
/// | 45  | PMODE0   | PHY mode    | PHY operation mode select bit 0 (pull-up) |
/// | 46  | NC       | No connect  | Do not connect        |
/// | 47  | NC       | No connect  | Do not connect        |
/// | 48  | AGND     | Analog ground |                       |
pub struct W5500 {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl W5500 {
    pub const TXN: copperleaf::PinRef = copperleaf::PinRef("TXN");
    pub const TXP: copperleaf::PinRef = copperleaf::PinRef("TXP");
    pub const AGND: copperleaf::PinRef = copperleaf::PinRef("AGND");
    pub const AVDD: copperleaf::PinRef = copperleaf::PinRef("AVDD");
    pub const RXN: copperleaf::PinRef = copperleaf::PinRef("RXN");
    pub const RXP: copperleaf::PinRef = copperleaf::PinRef("RXP");
    pub const DNC: copperleaf::PinRef = copperleaf::PinRef("DNC");
    pub const EXRES1: copperleaf::PinRef = copperleaf::PinRef("EXRES1");
    pub const NC: copperleaf::PinRef = copperleaf::PinRef("NC");
    pub const VBG: copperleaf::PinRef = copperleaf::PinRef("VBG");
    pub const TOCAP: copperleaf::PinRef = copperleaf::PinRef("TOCAP");
    pub const PIN_1V2O: copperleaf::PinRef = copperleaf::PinRef("1V2O");
    pub const RSVD: copperleaf::PinRef = copperleaf::PinRef("RSVD");
    pub const SPDLED: copperleaf::PinRef = copperleaf::PinRef("SPDLED");
    pub const LINKLED: copperleaf::PinRef = copperleaf::PinRef("LINKLED");
    pub const DUPLED: copperleaf::PinRef = copperleaf::PinRef("DUPLED");
    pub const ACTLED: copperleaf::PinRef = copperleaf::PinRef("ACTLED");
    pub const VDD: copperleaf::PinRef = copperleaf::PinRef("VDD");
    pub const GND: copperleaf::PinRef = copperleaf::PinRef("GND");
    pub const XI_CLKIN: copperleaf::PinRef = copperleaf::PinRef("XI/CLKIN");
    pub const XO: copperleaf::PinRef = copperleaf::PinRef("XO");
    pub const SCSn: copperleaf::PinRef = copperleaf::PinRef("SCSn");
    pub const SCLK: copperleaf::PinRef = copperleaf::PinRef("SCLK");
    pub const MISO: copperleaf::PinRef = copperleaf::PinRef("MISO");
    pub const MOSI: copperleaf::PinRef = copperleaf::PinRef("MOSI");
    pub const INTn: copperleaf::PinRef = copperleaf::PinRef("INTn");
    pub const RSTn: copperleaf::PinRef = copperleaf::PinRef("RSTn");
    pub const PMODE2: copperleaf::PinRef = copperleaf::PinRef("PMODE2");
    pub const PMODE1: copperleaf::PinRef = copperleaf::PinRef("PMODE1");
    pub const PMODE0: copperleaf::PinRef = copperleaf::PinRef("PMODE0");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("TXN").number("1").pos(-4.18, -2.75).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").role(Role::AnalogIn).rf_limits().pin(),
                Pin::build("TXP").number("2").pos(-4.18, -2.25).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").role(Role::AnalogIn).rf_limits().pin(),
                Pin::build("AGND").number("3").pos(-4.18, -1.75).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("AVDD").number("4").pos(-4.18, -1.25).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(2.97.volt(), 3.63.volt(), 0.15.amp()).pin(),
                Pin::build("RXN").number("5").pos(-4.18, -0.75).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").role(Role::AnalogIn).rf_limits().pin(),
                Pin::build("RXP").number("6").pos(-4.18, -0.25).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").role(Role::AnalogIn).rf_limits().pin(),
                Pin::build("DNC").number("7").pos(-4.18, 0.25).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("AVDD").number("8").pos(-4.18, 0.75).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(2.97.volt(), 3.63.volt(), 0.15.amp()).pin(),
                Pin::build("AGND").number("9").pos(-4.18, 1.25).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("EXRES1").number("10").pos(-4.18, 1.75).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").analog_in(),
                Pin::build("AVDD").number("11").pos(-4.18, 2.25).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(2.97.volt(), 3.63.volt(), 0.15.amp()).pin(),
                Pin::build("NC").number("12").pos(-4.18, 2.75).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("NC").number("13").pos(-2.75, 4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("AGND").number("14").pos(-2.25, 4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("AVDD").number("15").pos(-1.75, 4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(2.97.volt(), 3.63.volt(), 0.15.amp()).pin(),
                Pin::build("AGND").number("16").pos(-1.25, 4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("AVDD").number("17").pos(-0.75, 4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(2.97.volt(), 3.63.volt(), 0.15.amp()).pin(),
                Pin::build("VBG").number("18").pos(-0.25, 4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").analog_in(),
                Pin::build("AGND").number("19").pos(0.25, 4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("TOCAP").number("20").pos(0.75, 4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").analog_in(),
                Pin::build("AVDD").number("21").pos(1.25, 4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(2.97.volt(), 3.63.volt(), 0.15.amp()).pin(),
                Pin::build("1V2O").number("22").pos(1.75, 4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").role(Role::PowerOut).power_spec(PowerSpec { v_min: 1.2.volt(), v_max: 1.2.volt(), v_nom: Some(1.2.volt()), i_max: 0.01.amp() }).pin(),
                Pin::build("RSVD").number("23").pos(2.25, 4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("SPDLED").number("24").pos(2.75, 4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("LINKLED").number("25").pos(4.18, 2.75).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("DUPLED").number("26").pos(4.18, 2.25).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("ACTLED").number("27").pos(4.18, 1.75).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("VDD").number("28").pos(4.18, 1.25).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(2.97.volt(), 3.63.volt(), 0.15.amp()).pin(),
                Pin::build("GND").number("29").pos(4.18, 0.75).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("XI/CLKIN").number("30").pos(4.18, 0.25).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").clk(25.0),
                Pin::build("XO").number("31").pos(4.18, -0.25).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").clk(25.0),
                Pin::build("SCSn").number("32").pos(4.18, -0.75).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").spi(33.0),
                Pin::build("SCLK").number("33").pos(4.18, -1.25).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").clk(33.0),
                Pin::build("MISO").number("34").pos(4.18, -1.75).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").spi(33.0),
                Pin::build("MOSI").number("35").pos(4.18, -2.25).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").spi(33.0),
                Pin::build("INTn").number("36").pos(4.18, -2.75).rotation(0.0).length(1.56).width(1.56).height(0.28).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("RSTn").number("37").pos(2.75, -4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("RSVD").number("38").pos(2.25, -4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("RSVD").number("39").pos(1.75, -4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("RSVD").number("40").pos(1.25, -4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("RSVD").number("41").pos(0.75, -4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("RSVD").number("42").pos(0.25, -4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("PMODE2").number("43").pos(-0.25, -4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("PMODE1").number("44").pos(-0.75, -4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("PMODE0").number("45").pos(-1.25, -4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("NC").number("46").pos(-1.75, -4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("NC").number("47").pos(-2.25, -4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("AGND").number("48").pos(-2.75, -4.18).rotation(0.0).length(1.56).width(0.28).height(1.56).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
            ],
            mechanical: vec![
            ],
        }
    }

    pub fn constraints(&self) -> Vec<copperleaf::Constraint> {
        use copperleaf::{Constraint, units::UnitExt};
        vec![
            Constraint::Decoupling { values: vec![100.0.nf(), 10.0.uf()], per_pin: false },
            Constraint::MaxJunction { temp: 125.0.celsius() },
        ]
    }
}

impl copperleaf::Component for W5500 {
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
            symbol: Some("W5500".into()),
            
            footprint: Some("W5500".into()),
            
            datasheet: Some("https://docs.wiznet.io/img/products/w5500/W5500_ds_v110e.pdf".into()),
            
            description: Some("WIZnet W5500 Ethernet controller".into()),
            
            
            model_3d: None,
            model_3d_data: Some("<elided:2260308:3919b63dd7ca54c4>".into()),
            
            
            model_3d_rotation: (0.0, 0.0, 0.0),
            
            model_3d_offset: (0.0, 0.0, 0.0),
        })
    }
}

impl Default for W5500 {
    fn default() -> Self {
        Self::new()
    }
}
