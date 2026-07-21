/// Raspberry Pi RP2354A — RP2350 with 2 MB internal flash (QFN-60).
///
/// Raspberry Pi RP2354A microcontroller
///
/// Datasheet: https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | IOVDD    | Supply      | I/O supply            |
/// | 2   | GPIO0    | GPIO        |                       |
/// | 3   | GPIO1    | GPIO        |                       |
/// | 4   | GPIO2    | GPIO        |                       |
/// | 5   | GPIO3    | GPIO        |                       |
/// | 6   | DVDD     | Supply      | Core supply           |
/// | 7   | GPIO4    | GPIO        |                       |
/// | 8   | GPIO5    | GPIO        |                       |
/// | 9   | GPIO6    | GPIO        |                       |
/// | 10  | GPIO7    | GPIO        |                       |
/// | 11  | IOVDD    | Supply      | I/O supply            |
/// | 12  | GPIO8    | GPIO        |                       |
/// | 13  | GPIO9    | GPIO        |                       |
/// | 14  | GPIO10   | GPIO        |                       |
/// | 15  | GPIO11   | GPIO        |                       |
/// | 16  | GPIO12   | GPIO        |                       |
/// | 17  | GPIO13   | GPIO        |                       |
/// | 18  | GPIO14   | GPIO        |                       |
/// | 19  | GPIO15   | GPIO        |                       |
/// | 20  | IOVDD    | Supply      | I/O supply            |
/// | 21  | XIN      | Crystal     | 12 MHz crystal input  |
/// | 22  | XOUT     | Crystal     | 12 MHz crystal output |
/// | 23  | DVDD     | Supply      | Core supply           |
/// | 24  | SWCLK    | SWD         |                       |
/// | 25  | SWDIO    | SWD         |                       |
/// | 26  | RUN      | Reset       |                       |
/// | 27  | GPIO16   | GPIO        |                       |
/// | 28  | GPIO17   | GPIO        |                       |
/// | 29  | GPIO18   | GPIO        |                       |
/// | 30  | IOVDD    | Supply      | I/O supply            |
/// | 31  | GPIO19   | GPIO        |                       |
/// | 32  | GPIO20   | GPIO        |                       |
/// | 33  | GPIO21   | GPIO        |                       |
/// | 34  | GPIO22   | GPIO        |                       |
/// | 35  | GPIO23   | GPIO        |                       |
/// | 36  | GPIO24   | GPIO        |                       |
/// | 37  | GPIO25   | GPIO        |                       |
/// | 38  | IOVDD    | Supply      | I/O supply            |
/// | 39  | DVDD     | Supply      | Core supply           |
/// | 40  | GPIO26_ADC0 | GPIO/ADC    |                       |
/// | 41  | GPIO27_ADC1 | GPIO/ADC    |                       |
/// | 42  | GPIO28_ADC2 | GPIO/ADC    |                       |
/// | 43  | GPIO29_ADC3 | GPIO/ADC    |                       |
/// | 44  | ADC_AVDD | Supply      | ADC supply            |
/// | 45  | IOVDD    | Supply      | I/O supply            |
/// | 46  | VREG_AVDD | Supply      | Voltage regulator supply |
/// | 47  | VREG_PGND | Ground      |                       |
/// | 48  | VREG_LX  | Supply      | Voltage regulator switch node |
/// | 49  | VREG_VIN | Supply      | Voltage regulator input |
/// | 50  | VREG_FB  | Supply      | Voltage regulator feedback |
/// | 51  | USB_DM   | USB         |                       |
/// | 52  | USB_DP   | USB         |                       |
/// | 53  | USB_OTP_VDD | Supply      | USB OTP supply        |
/// | 54  | QSPI_IOVDD | Supply      | QSPI I/O supply       |
/// | 55  | QSPI_SD3 | QSPI        |                       |
/// | 56  | QSPI_SCLK | QSPI        |                       |
/// | 57  | QSPI_SD0 | QSPI        |                       |
/// | 58  | QSPI_SD2 | QSPI        |                       |
/// | 59  | QSPI_SD1 | QSPI        |                       |
/// | 60  | QSPI_SS  | QSPI        |                       |
/// | 61  | GND      | Ground      |                       |
pub struct Rp2354a {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Rp2354a {
    pub const IOVDD: copperleaf::PinRef = copperleaf::PinRef("IOVDD");
    pub const GPIO0: copperleaf::PinRef = copperleaf::PinRef("GPIO0");
    pub const GPIO1: copperleaf::PinRef = copperleaf::PinRef("GPIO1");
    pub const GPIO2: copperleaf::PinRef = copperleaf::PinRef("GPIO2");
    pub const GPIO3: copperleaf::PinRef = copperleaf::PinRef("GPIO3");
    pub const DVDD: copperleaf::PinRef = copperleaf::PinRef("DVDD");
    pub const GPIO4: copperleaf::PinRef = copperleaf::PinRef("GPIO4");
    pub const GPIO5: copperleaf::PinRef = copperleaf::PinRef("GPIO5");
    pub const GPIO6: copperleaf::PinRef = copperleaf::PinRef("GPIO6");
    pub const GPIO7: copperleaf::PinRef = copperleaf::PinRef("GPIO7");
    pub const GPIO8: copperleaf::PinRef = copperleaf::PinRef("GPIO8");
    pub const GPIO9: copperleaf::PinRef = copperleaf::PinRef("GPIO9");
    pub const GPIO10: copperleaf::PinRef = copperleaf::PinRef("GPIO10");
    pub const GPIO11: copperleaf::PinRef = copperleaf::PinRef("GPIO11");
    pub const GPIO12: copperleaf::PinRef = copperleaf::PinRef("GPIO12");
    pub const GPIO13: copperleaf::PinRef = copperleaf::PinRef("GPIO13");
    pub const GPIO14: copperleaf::PinRef = copperleaf::PinRef("GPIO14");
    pub const GPIO15: copperleaf::PinRef = copperleaf::PinRef("GPIO15");
    pub const XIN: copperleaf::PinRef = copperleaf::PinRef("XIN");
    pub const XOUT: copperleaf::PinRef = copperleaf::PinRef("XOUT");
    pub const SWCLK: copperleaf::PinRef = copperleaf::PinRef("SWCLK");
    pub const SWDIO: copperleaf::PinRef = copperleaf::PinRef("SWDIO");
    pub const RUN: copperleaf::PinRef = copperleaf::PinRef("RUN");
    pub const GPIO16: copperleaf::PinRef = copperleaf::PinRef("GPIO16");
    pub const GPIO17: copperleaf::PinRef = copperleaf::PinRef("GPIO17");
    pub const GPIO18: copperleaf::PinRef = copperleaf::PinRef("GPIO18");
    pub const GPIO19: copperleaf::PinRef = copperleaf::PinRef("GPIO19");
    pub const GPIO20: copperleaf::PinRef = copperleaf::PinRef("GPIO20");
    pub const GPIO21: copperleaf::PinRef = copperleaf::PinRef("GPIO21");
    pub const GPIO22: copperleaf::PinRef = copperleaf::PinRef("GPIO22");
    pub const GPIO23: copperleaf::PinRef = copperleaf::PinRef("GPIO23");
    pub const GPIO24: copperleaf::PinRef = copperleaf::PinRef("GPIO24");
    pub const GPIO25: copperleaf::PinRef = copperleaf::PinRef("GPIO25");
    pub const GPIO26_ADC0: copperleaf::PinRef = copperleaf::PinRef("GPIO26_ADC0");
    pub const GPIO27_ADC1: copperleaf::PinRef = copperleaf::PinRef("GPIO27_ADC1");
    pub const GPIO28_ADC2: copperleaf::PinRef = copperleaf::PinRef("GPIO28_ADC2");
    pub const GPIO29_ADC3: copperleaf::PinRef = copperleaf::PinRef("GPIO29_ADC3");
    pub const ADC_AVDD: copperleaf::PinRef = copperleaf::PinRef("ADC_AVDD");
    pub const VREG_AVDD: copperleaf::PinRef = copperleaf::PinRef("VREG_AVDD");
    pub const VREG_PGND: copperleaf::PinRef = copperleaf::PinRef("VREG_PGND");
    pub const VREG_LX: copperleaf::PinRef = copperleaf::PinRef("VREG_LX");
    pub const VREG_VIN: copperleaf::PinRef = copperleaf::PinRef("VREG_VIN");
    pub const VREG_FB: copperleaf::PinRef = copperleaf::PinRef("VREG_FB");
    pub const USB_DM: copperleaf::PinRef = copperleaf::PinRef("USB_DM");
    pub const USB_DP: copperleaf::PinRef = copperleaf::PinRef("USB_DP");
    pub const USB_OTP_VDD: copperleaf::PinRef = copperleaf::PinRef("USB_OTP_VDD");
    pub const QSPI_IOVDD: copperleaf::PinRef = copperleaf::PinRef("QSPI_IOVDD");
    pub const QSPI_SD3: copperleaf::PinRef = copperleaf::PinRef("QSPI_SD3");
    pub const QSPI_SCLK: copperleaf::PinRef = copperleaf::PinRef("QSPI_SCLK");
    pub const QSPI_SD0: copperleaf::PinRef = copperleaf::PinRef("QSPI_SD0");
    pub const QSPI_SD2: copperleaf::PinRef = copperleaf::PinRef("QSPI_SD2");
    pub const QSPI_SD1: copperleaf::PinRef = copperleaf::PinRef("QSPI_SD1");
    pub const QSPI_SS: copperleaf::PinRef = copperleaf::PinRef("QSPI_SS");
    pub const GND: copperleaf::PinRef = copperleaf::PinRef("GND");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("IOVDD").number("1").pos(-3.45, -2.8).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr(1.8.volt(), 3.3.volt(), 0.1.amp()).pin(),
                Pin::build("GPIO0").number("2").pos(-3.45, -2.4).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO1").number("3").pos(-3.45, -2.0).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO2").number("4").pos(-3.45, -1.6).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO3").number("5").pos(-3.45, -1.2).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("DVDD").number("6").pos(-3.45, -0.8).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr(1.1.volt(), 1.1.volt(), 0.1.amp()).pin(),
                Pin::build("GPIO4").number("7").pos(-3.45, -0.4).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO5").number("8").pos(-3.45, 0.0).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO6").number("9").pos(-3.45, 0.4).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO7").number("10").pos(-3.45, 0.8).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("IOVDD").number("11").pos(-3.45, 1.2).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr(1.8.volt(), 3.3.volt(), 0.1.amp()).pin(),
                Pin::build("GPIO8").number("12").pos(-3.45, 1.6).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO9").number("13").pos(-3.45, 2.0).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO10").number("14").pos(-3.45, 2.4).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO11").number("15").pos(-3.45, 2.8).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO12").number("16").pos(-2.8, 3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO13").number("17").pos(-2.4, 3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO14").number("18").pos(-2.0, 3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO15").number("19").pos(-1.6, 3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("IOVDD").number("20").pos(-1.2, 3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr(1.8.volt(), 3.3.volt(), 0.1.amp()).pin(),
                Pin::build("XIN").number("21").pos(-0.8, 3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").clk(12.0),
                Pin::build("XOUT").number("22").pos(-0.4, 3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").clk(12.0),
                Pin::build("DVDD").number("23").pos(0.0, 3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr(1.1.volt(), 1.1.volt(), 0.1.amp()).pin(),
                Pin::build("SWCLK").number("24").pos(0.4, 3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").clk(1.0),
                Pin::build("SWDIO").number("25").pos(0.8, 3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("RUN").number("26").pos(1.2, 3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO16").number("27").pos(1.6, 3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO17").number("28").pos(2.0, 3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO18").number("29").pos(2.4, 3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("IOVDD").number("30").pos(2.8, 3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr(1.8.volt(), 3.3.volt(), 0.1.amp()).pin(),
                Pin::build("GPIO19").number("31").pos(3.45, 2.8).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO20").number("32").pos(3.45, 2.4).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO21").number("33").pos(3.45, 2.0).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO22").number("34").pos(3.45, 1.6).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO23").number("35").pos(3.45, 1.2).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO24").number("36").pos(3.45, 0.8).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO25").number("37").pos(3.45, 0.4).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("IOVDD").number("38").pos(3.45, 0.0).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr(1.8.volt(), 3.3.volt(), 0.1.amp()).pin(),
                Pin::build("DVDD").number("39").pos(3.45, -0.4).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr(1.1.volt(), 1.1.volt(), 0.1.amp()).pin(),
                Pin::build("GPIO26_ADC0").number("40").pos(3.45, -0.8).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO27_ADC1").number("41").pos(3.45, -1.2).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO28_ADC2").number("42").pos(3.45, -1.6).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO29_ADC3").number("43").pos(3.45, -2.0).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("ADC_AVDD").number("44").pos(3.45, -2.4).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr_fixed(3.3.volt(), 0.1.amp()).pin(),
                Pin::build("IOVDD").number("45").pos(3.45, -2.8).rotation(0.0).length(0.8).width(0.8).height(0.2).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr(1.8.volt(), 3.3.volt(), 0.1.amp()).pin(),
                Pin::build("VREG_AVDD").number("46").pos(2.8, -3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr(1.1.volt(), 1.1.volt(), 0.0.amp()).pin(),
                Pin::build("VREG_PGND").number("47").pos(2.4, -3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("VREG_LX").number("48").pos(2.0, -3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr(0.0.volt(), 5.5.volt(), 0.2.amp()).pin(),
                Pin::build("VREG_VIN").number("49").pos(1.6, -3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr(2.7.volt(), 5.5.volt(), 0.0.amp()).pin(),
                Pin::build("VREG_FB").number("50").pos(1.2, -3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr(1.1.volt(), 1.1.volt(), 0.0.amp()).pin(),
                Pin::build("USB_DM").number("51").pos(0.8, -3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("USB_DP").number("52").pos(0.4, -3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("USB_OTP_VDD").number("53").pos(0.0, -3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr_fixed(3.3.volt(), 0.1.amp()).pin(),
                Pin::build("QSPI_IOVDD").number("54").pos(-0.4, -3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").pwr(1.8.volt(), 3.3.volt(), 0.1.amp()).pin(),
                Pin::build("QSPI_SD3").number("55").pos(-0.8, -3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("QSPI_SCLK").number("56").pos(-1.2, -3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").clk(50.0),
                Pin::build("QSPI_SD0").number("57").pos(-1.6, -3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("QSPI_SD2").number("58").pos(-2.0, -3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("QSPI_SD1").number("59").pos(-2.4, -3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("QSPI_SS").number("60").pos(-2.8, -3.45).rotation(0.0).length(0.8).width(0.2).height(0.8).pad_type("smd").pad_shape("roundrect").roundrect_rratio(0.25).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GND").number("61").pos(0.0, 0.0).rotation(0.0).length(3.4).width(3.4).height(3.4).pad_type("smd").pad_shape("rect").layers("F.Cu F.Mask").gnd(),
            ],
            mechanical: vec![
                copperleaf::Pad { number: "".into(), pos: (-1.13, -1.13), rotation: 0.0, width: 0.91, height: 0.91, pad_type: copperleaf::PadType::Smd, pad_shape: copperleaf::PadShape::RoundRect, roundrect_rratio: Some(0.25), solder_mask_margin: None, layers: Some("F.Paste".into()), drill: None },
                copperleaf::Pad { number: "".into(), pos: (-1.13, 0.0), rotation: 0.0, width: 0.91, height: 0.91, pad_type: copperleaf::PadType::Smd, pad_shape: copperleaf::PadShape::RoundRect, roundrect_rratio: Some(0.25), solder_mask_margin: None, layers: Some("F.Paste".into()), drill: None },
                copperleaf::Pad { number: "".into(), pos: (-1.13, 1.13), rotation: 0.0, width: 0.91, height: 0.91, pad_type: copperleaf::PadType::Smd, pad_shape: copperleaf::PadShape::RoundRect, roundrect_rratio: Some(0.25), solder_mask_margin: None, layers: Some("F.Paste".into()), drill: None },
                copperleaf::Pad { number: "".into(), pos: (0.0, -1.13), rotation: 0.0, width: 0.91, height: 0.91, pad_type: copperleaf::PadType::Smd, pad_shape: copperleaf::PadShape::RoundRect, roundrect_rratio: Some(0.25), solder_mask_margin: None, layers: Some("F.Paste".into()), drill: None },
                copperleaf::Pad { number: "".into(), pos: (0.0, 0.0), rotation: 0.0, width: 0.91, height: 0.91, pad_type: copperleaf::PadType::Smd, pad_shape: copperleaf::PadShape::RoundRect, roundrect_rratio: Some(0.25), solder_mask_margin: None, layers: Some("F.Paste".into()), drill: None },
                copperleaf::Pad { number: "".into(), pos: (0.0, 1.13), rotation: 0.0, width: 0.91, height: 0.91, pad_type: copperleaf::PadType::Smd, pad_shape: copperleaf::PadShape::RoundRect, roundrect_rratio: Some(0.25), solder_mask_margin: None, layers: Some("F.Paste".into()), drill: None },
                copperleaf::Pad { number: "".into(), pos: (1.13, -1.13), rotation: 0.0, width: 0.91, height: 0.91, pad_type: copperleaf::PadType::Smd, pad_shape: copperleaf::PadShape::RoundRect, roundrect_rratio: Some(0.25), solder_mask_margin: None, layers: Some("F.Paste".into()), drill: None },
                copperleaf::Pad { number: "".into(), pos: (1.13, 0.0), rotation: 0.0, width: 0.91, height: 0.91, pad_type: copperleaf::PadType::Smd, pad_shape: copperleaf::PadShape::RoundRect, roundrect_rratio: Some(0.25), solder_mask_margin: None, layers: Some("F.Paste".into()), drill: None },
                copperleaf::Pad { number: "".into(), pos: (1.13, 1.13), rotation: 0.0, width: 0.91, height: 0.91, pad_type: copperleaf::PadType::Smd, pad_shape: copperleaf::PadShape::RoundRect, roundrect_rratio: Some(0.25), solder_mask_margin: None, layers: Some("F.Paste".into()), drill: None },
            ],
        }
    }

    pub fn constraints(&self) -> Vec<copperleaf::Constraint> {
        use copperleaf::{Constraint, units::UnitExt};
        vec![
            Constraint::LengthMatch { group: "SPI0_BUS".into(), skew_ps: 200.0 },
            Constraint::LengthMatch { group: "SPI1_BUS".into(), skew_ps: 500.0 },
            Constraint::MaxJunction { temp: 85.0.celsius() },
        ]
    }
}

impl copperleaf::Component for Rp2354a {
    fn pins(&self) -> &[copperleaf::Pin] {
        &self.pins
    }

    fn constraints(&self) -> Vec<copperleaf::Constraint> {
        Self::constraints(self)
    }

    fn mechanical(&self) -> &[copperleaf::Pad] {
        &self.mechanical
    }

    fn symbol(&self) -> Option<&'static str> {
        Some("RP2354A")
    }

    fn footprint(&self) -> Option<&'static str> {
        Some("RP2354A")
    }

    fn datasheet(&self) -> Option<&'static str> {
        Some("https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf")
    }

    fn model_3d_data(&self) -> Option<&'static str> {
        Some("<elided:2252196:22996e8efa2743cf>")
    }

    fn description(&self) -> Option<&'static str> {
        Some("Raspberry Pi RP2354A microcontroller")
    }
}

impl Default for Rp2354a {
    fn default() -> Self {
        Self::new()
    }
}
