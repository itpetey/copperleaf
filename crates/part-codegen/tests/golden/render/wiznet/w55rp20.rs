/// WIZnet W55RP20 System-in-Package (SiP) solution that integrates WIZnet's W5500 Ethernet controller with the RP2040 microcontroller (QFN68, 8x8mm)
///
/// WIZnet W55RP20 System-in-Package (SiP) with W5500 Ethernet controller and RP2040 microcontroller
///
/// Datasheet: https://docs.wiznet.io/Product/ioNIC/W55RP20/documents_md
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | TXN      | Ethernet PHY | Differential transmit (negative); TXP/TXN differential signal pair |
/// | 2   | TXP      | Ethernet PHY | Differential transmit (positive); TXP/TXN differential signal pair |
/// | 3   | AVDD     | Analog supply | TOE analog 3.3V power |
/// | 4   | RXN      | Ethernet PHY | Differential receive (negative); RXP/RXN differential signal pair |
/// | 5   | RXP      | Ethernet PHY | Differential receive (positive); RXP/RXN differential signal pair |
/// | 6   | AVDD__1  | Analog supply | TOE analog 3.3V power |
/// | 7   | EXRES1   | PHY bias    | External reference resistor; connect 12.4k to AGND |
/// | 8   | AVDD__2  | Analog supply | TOE analog 3.3V power |
/// | 9   | GPIO4    | GPIO        | General-purpose digital input and output |
/// | 10  | GPIO5    | GPIO        | General-purpose digital input and output |
/// | 11  | GPIO6    | GPIO        | General-purpose digital input and output |
/// | 12  | GPIO7    | GPIO        | General-purpose digital input and output |
/// | 13  | IOVDD    | IO supply   | Digital IO supply; nominal voltage 3.3V |
/// | 14  | GPIO8    | GPIO        | General-purpose digital input and output |
/// | 15  | GPIO9    | GPIO        | General-purpose digital input and output |
/// | 16  | GPIO10   | GPIO        | General-purpose digital input and output |
/// | 17  | GPIO11   | GPIO        | General-purpose digital input and output |
/// | 18  | GPIO12   | GPIO        | General-purpose digital input and output |
/// | 19  | GPIO13   | GPIO        | General-purpose digital input and output |
/// | 20  | GPIO14   | GPIO        | General-purpose digital input and output |
/// | 21  | GPIO15   | GPIO        | General-purpose digital input and output |
/// | 22  | TESTEN   | Factory test | Factory test mode pin; tie to GND |
/// | 23  | XIN      | Crystal     | Crystal input/output; 12MHz crystal or single-ended CMOS clock input (XOUT disconnected) |
/// | 24  | XOUT     | Crystal     | Crystal output; leave disconnected when XIN is driven by an external clock |
/// | 25  | IOVDD__1 | IO supply   | Digital IO supply; nominal voltage 3.3V |
/// | 26  | DVDD     | Core supply | Digital core power supply; nominal voltage 1.1V; can be connected to VREG_OUT |
/// | 27  | SWCLK    | Debug       | Serial Wire Debug clock |
/// | 28  | SWD      | Debug       | Serial Wire Debug data |
/// | 29  | RUN      | Reset       | Global asynchronous reset; reset when driven low, run when driven high; tie to IOVDD if unused |
/// | 30  | GPIO16   | GPIO        | General-purpose digital input and output |
/// | 31  | AVDD__3  | Analog supply | TOE analog 3.3V power |
/// | 32  | VBG      | Band gap reference | Band gap output; leave floating |
/// | 33  | TOCAP    | Reference capacitor | External reference capacitor; connect 4.7uF to AGND, keep trace short |
/// | 34  | AVDD__4  | Analog supply | TOE analog 3.3V power |
/// | 35  | 1V2O     | Regulator output | 1.2V regulator output; connect 10nF decoupling capacitor to GND |
/// | 36  | SPDLED   | Speed LED   | Low: 100Mbps, High: 10Mbps |
/// | 37  | LINKLED  | Link LED    | Low: link established, High: link not established |
/// | 38  | DUPLED   | Duplex LED  | Low: full-duplex, High: half-duplex |
/// | 39  | ACTLED   | Active LED  | Low: carrier sense during TX or RX activity |
/// | 40  | GPIO18   | GPIO        | General-purpose digital input and output |
/// | 41  | GPIO19   | GPIO        | General-purpose digital input and output |
/// | 42  | VDD      | Digital supply | TOE digital 3.3V power |
/// | 43  | IOVDD__2 | IO supply   | Digital IO supply; nominal voltage 3.3V |
/// | 44  | XI       | Crystal     | 25MHz crystal input; can be driven by an external clock |
/// | 45  | XO       | Crystal     | 25MHz crystal output  |
/// | 46  | GPIO26/ADC0 | GPIO/ADC    | General-purpose digital I/O with analogue-to-digital converter function (ADC0) |
/// | 47  | GPIO27/ADC1 | GPIO/ADC    | General-purpose digital I/O with analogue-to-digital converter function (ADC1) |
/// | 48  | GPIO28/ADC2 | GPIO/ADC    | General-purpose digital I/O with analogue-to-digital converter function (ADC2) |
/// | 49  | GPIO29/ADC3 | GPIO/ADC    | General-purpose digital I/O with analogue-to-digital converter function (ADC3) |
/// | 50  | PMODE2   | PHY mode    | PHY operation mode select bit 2; default pull-up (77k) |
/// | 51  | PMODE1   | PHY mode    | PHY operation mode select bit 1; default pull-up (77k) |
/// | 52  | PMODE0   | PHY mode    | PHY operation mode select bit 0; default pull-up (77k) |
/// | 53  | IOVDD__3 | IO supply   | Digital IO supply; nominal voltage 3.3V |
/// | 54  | ADC_AVDD | ADC supply  | Power supply for the analogue-to-digital converter; nominal voltage 3.3V |
/// | 55  | VREG_VIN | Regulator input | Voltage regulator input supply; nominal voltage 1.8V to 3.3V |
/// | 56  | VREG_OUT | Regulator output | Voltage regulator output for the internal core; nominal voltage 1.1V, 100mA max; connect to DVDD |
/// | 57  | USB_DM   | USB         | USB Full Speed data (D-); 27 ohm series termination required |
/// | 58  | UDB_DP   | USB         | USB Full Speed data (D+); 27 ohm series termination required |
/// | 59  | USB_VDD  | USB supply  | Power supply for the internal USB Full Speed PHY; nominal voltage 3.3V |
/// | 60  | IOVDD__4 | IO supply   | Digital IO supply; nominal voltage 3.3V |
/// | 61  | DVDD__1  | Core supply | Digital core power supply; nominal voltage 1.1V; can be connected to VREG_OUT |
/// | 62  | VDD__1   | Flash supply | Power supply of the on-chip flash memory |
/// | 63  | BOOT     | Boot mode   | When low during power-up/reset the device enters BOOTSEL mode |
/// | 64  | IOVDD__5 | IO supply   | Digital IO supply; nominal voltage 3.3V |
/// | 65  | GPIO0    | GPIO        | General-purpose digital input and output |
/// | 66  | GPIO1    | GPIO        | General-purpose digital input and output |
/// | 67  | GPIO2    | GPIO        | General-purpose digital input and output |
/// | 68  | GPIO3    | GPIO        | General-purpose digital input and output |
/// | 69  | GND      | Ground      |                       |
pub struct W55rp20 {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl W55rp20 {
    pub const TXN: copperleaf::PinRef = copperleaf::PinRef("1");
    pub const TXP: copperleaf::PinRef = copperleaf::PinRef("2");
    pub const AVDD: copperleaf::PinRef = copperleaf::PinRef("3");
    pub const RXN: copperleaf::PinRef = copperleaf::PinRef("4");
    pub const RXP: copperleaf::PinRef = copperleaf::PinRef("5");
    pub const AVDD__1: copperleaf::PinRef = copperleaf::PinRef("6");
    pub const EXRES1: copperleaf::PinRef = copperleaf::PinRef("7");
    pub const AVDD__2: copperleaf::PinRef = copperleaf::PinRef("8");
    pub const GPIO4: copperleaf::PinRef = copperleaf::PinRef("9");
    pub const GPIO5: copperleaf::PinRef = copperleaf::PinRef("10");
    pub const GPIO6: copperleaf::PinRef = copperleaf::PinRef("11");
    pub const GPIO7: copperleaf::PinRef = copperleaf::PinRef("12");
    pub const IOVDD: copperleaf::PinRef = copperleaf::PinRef("13");
    pub const GPIO8: copperleaf::PinRef = copperleaf::PinRef("14");
    pub const GPIO9: copperleaf::PinRef = copperleaf::PinRef("15");
    pub const GPIO10: copperleaf::PinRef = copperleaf::PinRef("16");
    pub const GPIO11: copperleaf::PinRef = copperleaf::PinRef("17");
    pub const GPIO12: copperleaf::PinRef = copperleaf::PinRef("18");
    pub const GPIO13: copperleaf::PinRef = copperleaf::PinRef("19");
    pub const GPIO14: copperleaf::PinRef = copperleaf::PinRef("20");
    pub const GPIO15: copperleaf::PinRef = copperleaf::PinRef("21");
    pub const TESTEN: copperleaf::PinRef = copperleaf::PinRef("22");
    pub const XIN: copperleaf::PinRef = copperleaf::PinRef("23");
    pub const XOUT: copperleaf::PinRef = copperleaf::PinRef("24");
    pub const IOVDD__1: copperleaf::PinRef = copperleaf::PinRef("25");
    pub const DVDD: copperleaf::PinRef = copperleaf::PinRef("26");
    pub const SWCLK: copperleaf::PinRef = copperleaf::PinRef("27");
    pub const SWD: copperleaf::PinRef = copperleaf::PinRef("28");
    pub const RUN: copperleaf::PinRef = copperleaf::PinRef("29");
    pub const GPIO16: copperleaf::PinRef = copperleaf::PinRef("30");
    pub const AVDD__3: copperleaf::PinRef = copperleaf::PinRef("31");
    pub const VBG: copperleaf::PinRef = copperleaf::PinRef("32");
    pub const TOCAP: copperleaf::PinRef = copperleaf::PinRef("33");
    pub const AVDD__4: copperleaf::PinRef = copperleaf::PinRef("34");
    pub const PIN_1V2O: copperleaf::PinRef = copperleaf::PinRef("35");
    pub const SPDLED: copperleaf::PinRef = copperleaf::PinRef("36");
    pub const LINKLED: copperleaf::PinRef = copperleaf::PinRef("37");
    pub const DUPLED: copperleaf::PinRef = copperleaf::PinRef("38");
    pub const ACTLED: copperleaf::PinRef = copperleaf::PinRef("39");
    pub const GPIO18: copperleaf::PinRef = copperleaf::PinRef("40");
    pub const GPIO19: copperleaf::PinRef = copperleaf::PinRef("41");
    pub const VDD: copperleaf::PinRef = copperleaf::PinRef("42");
    pub const IOVDD__2: copperleaf::PinRef = copperleaf::PinRef("43");
    pub const XI: copperleaf::PinRef = copperleaf::PinRef("44");
    pub const XO: copperleaf::PinRef = copperleaf::PinRef("45");
    pub const GPIO26_ADC0: copperleaf::PinRef = copperleaf::PinRef("46");
    pub const GPIO27_ADC1: copperleaf::PinRef = copperleaf::PinRef("47");
    pub const GPIO28_ADC2: copperleaf::PinRef = copperleaf::PinRef("48");
    pub const GPIO29_ADC3: copperleaf::PinRef = copperleaf::PinRef("49");
    pub const PMODE2: copperleaf::PinRef = copperleaf::PinRef("50");
    pub const PMODE1: copperleaf::PinRef = copperleaf::PinRef("51");
    pub const PMODE0: copperleaf::PinRef = copperleaf::PinRef("52");
    pub const IOVDD__3: copperleaf::PinRef = copperleaf::PinRef("53");
    pub const ADC_AVDD: copperleaf::PinRef = copperleaf::PinRef("54");
    pub const VREG_VIN: copperleaf::PinRef = copperleaf::PinRef("55");
    pub const VREG_OUT: copperleaf::PinRef = copperleaf::PinRef("56");
    pub const USB_DM: copperleaf::PinRef = copperleaf::PinRef("57");
    pub const UDB_DP: copperleaf::PinRef = copperleaf::PinRef("58");
    pub const USB_VDD: copperleaf::PinRef = copperleaf::PinRef("59");
    pub const IOVDD__4: copperleaf::PinRef = copperleaf::PinRef("60");
    pub const DVDD__1: copperleaf::PinRef = copperleaf::PinRef("61");
    pub const VDD__1: copperleaf::PinRef = copperleaf::PinRef("62");
    pub const BOOT: copperleaf::PinRef = copperleaf::PinRef("63");
    pub const IOVDD__5: copperleaf::PinRef = copperleaf::PinRef("64");
    pub const GPIO0: copperleaf::PinRef = copperleaf::PinRef("65");
    pub const GPIO1: copperleaf::PinRef = copperleaf::PinRef("66");
    pub const GPIO2: copperleaf::PinRef = copperleaf::PinRef("67");
    pub const GPIO3: copperleaf::PinRef = copperleaf::PinRef("68");
    pub const GND: copperleaf::PinRef = copperleaf::PinRef("69");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("TXN").number("1").pos(-3.95, -3.2).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").role(Role::AnalogIn).rf_limits().pin(),
                Pin::build("TXP").number("2").pos(-3.95, -2.8).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").role(Role::AnalogIn).rf_limits().pin(),
                Pin::build("AVDD").number("3").pos(-3.95, -2.4).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(2.97.volt(), 3.63.volt(), 0.15.amp()).pin(),
                Pin::build("RXN").number("4").pos(-3.95, -2.0).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").role(Role::AnalogIn).rf_limits().pin(),
                Pin::build("RXP").number("5").pos(-3.95, -1.6).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").role(Role::AnalogIn).rf_limits().pin(),
                Pin::build("AVDD__1").number("6").pos(-3.95, -1.2).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(2.97.volt(), 3.63.volt(), 0.15.amp()).pin(),
                Pin::build("EXRES1").number("7").pos(-3.95, -0.8).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").analog_in(),
                Pin::build("AVDD__2").number("8").pos(-3.95, -0.4).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(2.97.volt(), 3.63.volt(), 0.15.amp()).pin(),
                Pin::build("GPIO4").number("9").pos(-3.95, 0.0).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO5").number("10").pos(-3.95, 0.4).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO6").number("11").pos(-3.95, 0.8).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO7").number("12").pos(-3.95, 1.2).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("IOVDD").number("13").pos(-3.95, 1.6).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(1.62.volt(), 3.63.volt(), 0.05.amp()).pin(),
                Pin::build("GPIO8").number("14").pos(-3.95, 2.0).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO9").number("15").pos(-3.95, 2.4).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO10").number("16").pos(-3.95, 2.8).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO11").number("17").pos(-3.95, 3.2).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO12").number("18").pos(-3.2, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO13").number("19").pos(-2.8, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO14").number("20").pos(-2.4, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO15").number("21").pos(-2.0, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("TESTEN").number("22").pos(-1.6, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("XIN").number("23").pos(-1.2, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").clk(12.0),
                Pin::build("XOUT").number("24").pos(-0.8, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").clk(12.0),
                Pin::build("IOVDD__1").number("25").pos(-0.4, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(1.62.volt(), 3.63.volt(), 0.05.amp()).pin(),
                Pin::build("DVDD").number("26").pos(0.0, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(1.05.volt(), 1.16.volt(), 0.1.amp()).pin(),
                Pin::build("SWCLK").number("27").pos(0.4, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("SWD").number("28").pos(0.8, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("RUN").number("29").pos(1.2, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO16").number("30").pos(1.6, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("AVDD__3").number("31").pos(2.0, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(2.97.volt(), 3.63.volt(), 0.15.amp()).pin(),
                Pin::build("VBG").number("32").pos(2.4, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").analog_in(),
                Pin::build("TOCAP").number("33").pos(2.8, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").analog_in(),
                Pin::build("AVDD__4").number("34").pos(3.2, 3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(2.97.volt(), 3.63.volt(), 0.15.amp()).pin(),
                Pin::build("1V2O").number("35").pos(3.95, 3.2).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").role(Role::AnalogIn).rf_limits().pin(),
                Pin::build("SPDLED").number("36").pos(3.95, 2.8).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("LINKLED").number("37").pos(3.95, 2.4).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("DUPLED").number("38").pos(3.95, 2.0).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("ACTLED").number("39").pos(3.95, 1.6).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO18").number("40").pos(3.95, 1.2).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO19").number("41").pos(3.95, 0.8).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("VDD").number("42").pos(3.95, 0.4).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(2.97.volt(), 3.63.volt(), 0.15.amp()).pin(),
                Pin::build("IOVDD__2").number("43").pos(3.95, 0.0).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(1.62.volt(), 3.63.volt(), 0.05.amp()).pin(),
                Pin::build("XI").number("44").pos(3.95, -0.4).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").clk(25.0),
                Pin::build("XO").number("45").pos(3.95, -0.8).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").clk(25.0),
                Pin::build("GPIO26/ADC0").number("46").pos(3.95, -1.2).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO27/ADC1").number("47").pos(3.95, -1.6).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO28/ADC2").number("48").pos(3.95, -2.0).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO29/ADC3").number("49").pos(3.95, -2.4).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("PMODE2").number("50").pos(3.95, -2.8).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("PMODE1").number("51").pos(3.95, -3.2).rotation(0.0).length(0.81).width(0.81).height(0.22).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("PMODE0").number("52").pos(3.2, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("IOVDD__3").number("53").pos(2.8, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(1.62.volt(), 3.63.volt(), 0.05.amp()).pin(),
                Pin::build("ADC_AVDD").number("54").pos(2.4, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(1.62.volt(), 3.63.volt(), 0.05.amp()).pin(),
                Pin::build("VREG_VIN").number("55").pos(2.0, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(1.62.volt(), 3.63.volt(), 0.15.amp()).pin(),
                Pin::build("VREG_OUT").number("56").pos(1.6, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr_fixed(1.1.volt(), 0.1.amp()).pin(),
                Pin::build("USB_DM").number("57").pos(1.2, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("UDB_DP").number("58").pos(0.8, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("USB_VDD").number("59").pos(0.4, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(3.135.volt(), 3.63.volt(), 0.1.amp()).pin(),
                Pin::build("IOVDD__4").number("60").pos(0.0, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(1.62.volt(), 3.63.volt(), 0.05.amp()).pin(),
                Pin::build("DVDD__1").number("61").pos(-0.4, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(1.05.volt(), 1.16.volt(), 0.1.amp()).pin(),
                Pin::build("VDD__1").number("62").pos(-0.8, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(1.8.volt(), 3.3.volt(), 0.05.amp()).pin(),
                Pin::build("BOOT").number("63").pos(-1.2, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("IOVDD__5").number("64").pos(-1.6, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(1.62.volt(), 3.63.volt(), 0.05.amp()).pin(),
                Pin::build("GPIO0").number("65").pos(-2.0, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO1").number("66").pos(-2.4, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO2").number("67").pos(-2.8, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO3").number("68").pos(-3.2, -3.95).rotation(0.0).length(0.81).width(0.22).height(0.81).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.125).solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GND").number("69").pos(0.0, 0.0).rotation(0.0).length(6.2).width(6.2).height(6.2).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask").gnd(),
            ],
            mechanical: vec![
            ],
        }
    }

    pub fn constraints(&self) -> Vec<copperleaf::Constraint> {
        use copperleaf::{Constraint, units::UnitExt};
        vec![
            Constraint::MaxJunction { temp: 85.0.celsius() },
        ]
    }

    pub fn layout_constraints(&self) -> Vec<copperleaf::LayoutConstraint> {
        use copperleaf::units::UnitExt;
        vec![
        ]
    }
}

impl copperleaf::Component for W55rp20 {
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
            symbol: Some("W55RP20".into()),
            
            footprint: Some("W55RP20".into()),
            
            datasheet: Some("https://docs.wiznet.io/Product/ioNIC/W55RP20/documents_md".into()),
            
            description: Some("WIZnet W55RP20 System-in-Package (SiP) with W5500 Ethernet controller and RP2040 microcontroller".into()),
            
            
            model_3d: None,
            model_3d_data: Some("<elided:756680:789986df5197a7f5>".into()),
            
            
            model_3d_rotation: (0.0, 0.0, 0.0),
            
            model_3d_offset: (0.0, 0.0, 0.0),
            
            fab_extent: None,
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for W55rp20 {
    fn default() -> Self {
        Self::new()
    }
}
