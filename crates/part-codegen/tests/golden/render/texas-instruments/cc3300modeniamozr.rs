/// Texas Instruments CC3300MODENIAMOZR 2.4GHz Wi-Fi 6 Companion Module (65-pin LGA)
///
/// Datasheet: https://www.ti.com/lit/gpn/cc3300mod
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | GND      | Ground      |                       |
/// | 2   | GND_1    | Ground      |                       |
/// | 3   | SDIO_CLK | Input       | SDIO clock or SPI clock |
/// | 4   | SDIO_CMD | I/O         | SDIO command or SPI PICO |
/// | 5   | SDIO_D3  | I/O         | SDIO data D3 or SPI CS |
/// | 6   | SDIO_D2  | I/O         | SDIO data D2          |
/// | 7   | SDIO_D1  | I/O         | SDIO data D1          |
/// | 8   | SDIO_D0  | I/O         | SDIO data D0 or SPI POCI |
/// | 9   | LOGGER   | Output      | Tracer (UART TX debug logger) |
/// | 10  | HOST_IRQ_WL | Output      | Interrupt request to host for WLAN |
/// | 11  | HOST_IRQ_BLE | Output      | Reserved for future use |
/// | 12  | SWDIO    | I/O         | Serial wire debug I/O |
/// | 13  | SWCLK    | Input       | Serial wire debug clock |
/// | 14  | GND_2    | Ground      |                       |
/// | 15  | GND_3    | Ground      |                       |
/// | 16  | GND_4    | Ground      |                       |
/// | 17  | NRESET   | Input       | Reset line for enabling or disabling device (active low) |
/// | 18  | SLOW_CLK_IN | Input       | 32.768kHz RTC clock input |
/// | 19  | VPP_IN   | Supply      | OTP programming input supply |
/// | 20  | GND_5    | Ground      |                       |
/// | 21  | GND_6    | Ground      |                       |
/// | 22  | GND_7    | Ground      |                       |
/// | 23  | RF_OUT   | RF          | Bluetooth Low Energy and WLAN 2.4GHz RF port |
/// | 24  | GND_8    | Ground      |                       |
/// | 25  | GND_9    | Ground      |                       |
/// | 26  | GND_10   | Ground      |                       |
/// | 27  | GND_11   | Ground      |                       |
/// | 28  | GND_12   | Ground      |                       |
/// | 29  | GND_13   | Ground      |                       |
/// | 30  | GND_14   | Ground      |                       |
/// | 31  | GND_15   | Ground      |                       |
/// | 32  | GND_16   | Ground      |                       |
/// | 33  | GND_17   | Ground      |                       |
/// | 34  | GND_18   | Ground      |                       |
/// | 35  | GND_19   | Ground      |                       |
/// | 36  | GND_20   | Ground      |                       |
/// | 37  | 3V3_IN   | Supply      | PA voltage            |
/// | 38  | 3V3_IN_1 | Supply      | PA voltage            |
/// | 39  | GND_21   | Ground      |                       |
/// | 40  | GND_22   | Ground      |                       |
/// | 41  | GND_23   | Ground      |                       |
/// | 42  | GND_24   | Ground      |                       |
/// | 43  | GND_25   | Ground      |                       |
/// | 44  | GND_26   | Ground      |                       |
/// | 45  | COEX_PRIORITY | Input       | External coexistence interface: priority |
/// | 46  | COEX_REQ | Input       | External coexistence interface: request |
/// | 47  | COEX_GRANT | Output      | External coexistence interface: grant |
/// | 48  | UART_RTS | Output      | Device RTS signal: flow control for BLE HCI |
/// | 49  | UART_CTS | Input       | Device CTS signal: flow control for BLE HCI |
/// | 50  | UART_RX  | Input       | UART RX for BLE HCI   |
/// | 51  | UART_TX  | Output      | UART TX for BLE HCI   |
/// | 52  | ANT_SEL  | Output      | Antenna select control line |
/// | 53  | 1V8_IN   | Supply      | Main supply voltage for analog and digital (VDD_MAIN_IN, VDDA_IN1, VDDA_IN2, VIO) |
/// | 54  | 1V8_IN_1 | Supply      | Main supply voltage for analog and digital (VDD_MAIN_IN, VDDA_IN1, VDDA_IN2, VIO) |
/// | 55  | GND_27   | Ground      |                       |
/// | 56  | GND_28   | Ground      |                       |
/// | 57  | GND_29   | Ground      |                       |
/// | 58  | GND_30   | Ground      |                       |
/// | 59  | GND_31   | Ground      |                       |
/// | 60  | GND_32   | Ground      |                       |
/// | 61  | GND_33   | Ground      |                       |
/// | 62  | GND_34   | Ground      |                       |
/// | 63  | GND_35   | Ground      |                       |
/// | 64  | GND_36   | Ground      |                       |
/// | 65  | GND_37   | Ground      |                       |
pub struct Cc3300modeniamozr {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Cc3300modeniamozr {
    pub const GND: copperleaf::PinRef = copperleaf::PinRef("GND");
    pub const GND_1: copperleaf::PinRef = copperleaf::PinRef("GND_1");
    pub const SDIO_CLK: copperleaf::PinRef = copperleaf::PinRef("SDIO_CLK");
    pub const SDIO_CMD: copperleaf::PinRef = copperleaf::PinRef("SDIO_CMD");
    pub const SDIO_D3: copperleaf::PinRef = copperleaf::PinRef("SDIO_D3");
    pub const SDIO_D2: copperleaf::PinRef = copperleaf::PinRef("SDIO_D2");
    pub const SDIO_D1: copperleaf::PinRef = copperleaf::PinRef("SDIO_D1");
    pub const SDIO_D0: copperleaf::PinRef = copperleaf::PinRef("SDIO_D0");
    pub const LOGGER: copperleaf::PinRef = copperleaf::PinRef("LOGGER");
    pub const HOST_IRQ_WL: copperleaf::PinRef = copperleaf::PinRef("HOST_IRQ_WL");
    pub const HOST_IRQ_BLE: copperleaf::PinRef = copperleaf::PinRef("HOST_IRQ_BLE");
    pub const SWDIO: copperleaf::PinRef = copperleaf::PinRef("SWDIO");
    pub const SWCLK: copperleaf::PinRef = copperleaf::PinRef("SWCLK");
    pub const GND_2: copperleaf::PinRef = copperleaf::PinRef("GND_2");
    pub const GND_3: copperleaf::PinRef = copperleaf::PinRef("GND_3");
    pub const GND_4: copperleaf::PinRef = copperleaf::PinRef("GND_4");
    pub const NRESET: copperleaf::PinRef = copperleaf::PinRef("NRESET");
    pub const SLOW_CLK_IN: copperleaf::PinRef = copperleaf::PinRef("SLOW_CLK_IN");
    pub const VPP_IN: copperleaf::PinRef = copperleaf::PinRef("VPP_IN");
    pub const GND_5: copperleaf::PinRef = copperleaf::PinRef("GND_5");
    pub const GND_6: copperleaf::PinRef = copperleaf::PinRef("GND_6");
    pub const GND_7: copperleaf::PinRef = copperleaf::PinRef("GND_7");
    pub const RF_OUT: copperleaf::PinRef = copperleaf::PinRef("RF_OUT");
    pub const GND_8: copperleaf::PinRef = copperleaf::PinRef("GND_8");
    pub const GND_9: copperleaf::PinRef = copperleaf::PinRef("GND_9");
    pub const GND_10: copperleaf::PinRef = copperleaf::PinRef("GND_10");
    pub const GND_11: copperleaf::PinRef = copperleaf::PinRef("GND_11");
    pub const GND_12: copperleaf::PinRef = copperleaf::PinRef("GND_12");
    pub const GND_13: copperleaf::PinRef = copperleaf::PinRef("GND_13");
    pub const GND_14: copperleaf::PinRef = copperleaf::PinRef("GND_14");
    pub const GND_15: copperleaf::PinRef = copperleaf::PinRef("GND_15");
    pub const GND_16: copperleaf::PinRef = copperleaf::PinRef("GND_16");
    pub const GND_17: copperleaf::PinRef = copperleaf::PinRef("GND_17");
    pub const GND_18: copperleaf::PinRef = copperleaf::PinRef("GND_18");
    pub const GND_19: copperleaf::PinRef = copperleaf::PinRef("GND_19");
    pub const GND_20: copperleaf::PinRef = copperleaf::PinRef("GND_20");
    pub const PIN_3V3_IN: copperleaf::PinRef = copperleaf::PinRef("3V3_IN");
    pub const PIN_3V3_IN_1: copperleaf::PinRef = copperleaf::PinRef("3V3_IN_1");
    pub const GND_21: copperleaf::PinRef = copperleaf::PinRef("GND_21");
    pub const GND_22: copperleaf::PinRef = copperleaf::PinRef("GND_22");
    pub const GND_23: copperleaf::PinRef = copperleaf::PinRef("GND_23");
    pub const GND_24: copperleaf::PinRef = copperleaf::PinRef("GND_24");
    pub const GND_25: copperleaf::PinRef = copperleaf::PinRef("GND_25");
    pub const GND_26: copperleaf::PinRef = copperleaf::PinRef("GND_26");
    pub const COEX_PRIORITY: copperleaf::PinRef = copperleaf::PinRef("COEX_PRIORITY");
    pub const COEX_REQ: copperleaf::PinRef = copperleaf::PinRef("COEX_REQ");
    pub const COEX_GRANT: copperleaf::PinRef = copperleaf::PinRef("COEX_GRANT");
    pub const UART_RTS: copperleaf::PinRef = copperleaf::PinRef("UART_RTS");
    pub const UART_CTS: copperleaf::PinRef = copperleaf::PinRef("UART_CTS");
    pub const UART_RX: copperleaf::PinRef = copperleaf::PinRef("UART_RX");
    pub const UART_TX: copperleaf::PinRef = copperleaf::PinRef("UART_TX");
    pub const ANT_SEL: copperleaf::PinRef = copperleaf::PinRef("ANT_SEL");
    pub const PIN_1V8_IN: copperleaf::PinRef = copperleaf::PinRef("1V8_IN");
    pub const PIN_1V8_IN_1: copperleaf::PinRef = copperleaf::PinRef("1V8_IN_1");
    pub const GND_27: copperleaf::PinRef = copperleaf::PinRef("GND_27");
    pub const GND_28: copperleaf::PinRef = copperleaf::PinRef("GND_28");
    pub const GND_29: copperleaf::PinRef = copperleaf::PinRef("GND_29");
    pub const GND_30: copperleaf::PinRef = copperleaf::PinRef("GND_30");
    pub const GND_31: copperleaf::PinRef = copperleaf::PinRef("GND_31");
    pub const GND_32: copperleaf::PinRef = copperleaf::PinRef("GND_32");
    pub const GND_33: copperleaf::PinRef = copperleaf::PinRef("GND_33");
    pub const GND_34: copperleaf::PinRef = copperleaf::PinRef("GND_34");
    pub const GND_35: copperleaf::PinRef = copperleaf::PinRef("GND_35");
    pub const GND_36: copperleaf::PinRef = copperleaf::PinRef("GND_36");
    pub const GND_37: copperleaf::PinRef = copperleaf::PinRef("GND_37");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("GND").number("1").pos(-4.99999, -5.0).rotation(0.0).length(0.508).width(0.508).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_1").number("2").pos(-5.0, -3.9).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("SDIO_CLK").number("3").pos(-5.0, -3.25).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("SDIO_CMD").number("4").pos(-5.0, -2.6).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("SDIO_D3").number("5").pos(-5.0, -1.95).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("SDIO_D2").number("6").pos(-5.0, -1.3).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("SDIO_D1").number("7").pos(-5.0, -0.65).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("SDIO_D0").number("8").pos(-5.0, 0.0).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("LOGGER").number("9").pos(-5.0, 0.65).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("HOST_IRQ_WL").number("10").pos(-5.0, 1.3).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("HOST_IRQ_BLE").number("11").pos(-5.0, 1.95).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("SWDIO").number("12").pos(-5.0, 2.6).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("SWCLK").number("13").pos(-5.0, 3.25).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("GND_2").number("14").pos(-5.0, 3.9).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_3").number("15").pos(-4.99999, 5.0).rotation(0.0).length(0.508).width(0.508).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_4").number("16").pos(-3.9, 5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("NRESET").number("17").pos(-3.250001, 5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("SLOW_CLK_IN").number("18").pos(-2.6, 5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("VPP_IN").number("19").pos(-1.950001, 5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").pwr(1.62.volt(), 1.98.volt(), 185.0.amp()).pin(),
                Pin::build("GND_5").number("20").pos(-1.3, 5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_6").number("21").pos(-0.650001, 5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_7").number("22").pos(0.0, 5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("RF_OUT").number("23").pos(0.649999, 5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").role(Role::AnalogIn).rf_limits().pin(),
                Pin::build("GND_8").number("24").pos(1.3, 5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_9").number("25").pos(1.949999, 5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_10").number("26").pos(2.6, 5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_11").number("27").pos(3.249999, 5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_12").number("28").pos(3.9, 5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_13").number("29").pos(4.99999, 5.0).rotation(0.0).length(0.508).width(0.508).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_14").number("30").pos(5.0, 3.9).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_15").number("31").pos(5.0, 3.25).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_16").number("32").pos(5.0, 2.6).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_17").number("33").pos(5.0, 1.95).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_18").number("34").pos(5.0, 1.3).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_19").number("35").pos(5.0, 0.65).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_20").number("36").pos(5.0, 0.0).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("3V3_IN").number("37").pos(5.0, -0.65).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").pwr(2.97.volt(), 3.63.volt(), 340.0.amp()).pin(),
                Pin::build("3V3_IN_1").number("38").pos(5.0, -1.3).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").pwr(2.97.volt(), 3.63.volt(), 340.0.amp()).pin(),
                Pin::build("GND_21").number("39").pos(5.0, -1.95).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_22").number("40").pos(5.0, -2.6).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_23").number("41").pos(5.0, -3.25).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_24").number("42").pos(5.0, -3.9).rotation(0.0).length(0.5).width(0.5).height(0.3).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_25").number("43").pos(4.99999, -5.0).rotation(0.0).length(0.508).width(0.508).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_26").number("44").pos(3.9, -5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("COEX_PRIORITY").number("45").pos(3.249999, -5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("COEX_REQ").number("46").pos(2.6, -5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("COEX_GRANT").number("47").pos(1.949999, -5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("UART_RTS").number("48").pos(1.3, -5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("UART_CTS").number("49").pos(0.649999, -5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("UART_RX").number("50").pos(0.0, -5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("UART_TX").number("51").pos(-0.650001, -5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("ANT_SEL").number("52").pos(-1.3, -5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("1V8_IN").number("53").pos(-1.950001, -5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").pwr(1.62.volt(), 1.98.volt(), 185.0.amp()).pin(),
                Pin::build("1V8_IN_1").number("54").pos(-2.6, -5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").pwr(1.62.volt(), 1.98.volt(), 185.0.amp()).pin(),
                Pin::build("GND_27").number("55").pos(-3.250001, -5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_28").number("56").pos(-3.9, -5.0).rotation(0.0).length(0.508).width(0.3048).height(0.508).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_29").number("57").pos(-2.600086, -2.199999).rotation(0.0).length(0.9906).width(0.9906).height(0.9906).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_30").number("58").pos(-2.600086, -0.749999).rotation(0.0).length(0.9906).width(0.9906).height(0.9906).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_31").number("59").pos(-2.600086, 0.700001).rotation(0.0).length(0.9906).width(0.9906).height(0.9906).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_32").number("60").pos(-1.15, -2.199999).rotation(0.0).length(0.9906).width(0.9906).height(0.9906).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_33").number("61").pos(-1.15, -0.749999).rotation(0.0).length(0.9906).width(0.9906).height(0.9906).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_34").number("62").pos(-1.15, 0.700001).rotation(0.0).length(0.9906).width(0.9906).height(0.9906).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_35").number("63").pos(0.300086, -2.199999).rotation(0.0).length(0.9906).width(0.9906).height(0.9906).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_36").number("64").pos(0.300086, -0.749999).rotation(0.0).length(0.9906).width(0.9906).height(0.9906).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_37").number("65").pos(0.300086, 0.700001).rotation(0.0).length(0.9906).width(0.9906).height(0.9906).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
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

impl copperleaf::Component for Cc3300modeniamozr {
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
            symbol: Some("CC3300MODENIAMOZR".into()),
            
            footprint: Some("CC3300MODENIAMOZR".into()),
            
            datasheet: Some("https://www.ti.com/lit/gpn/cc3300mod".into()),
            
            
            description: None,
            
            model_3d: None,
            model_3d_data: Some("<elided:1538000:e781a4ce9825c181>".into()),
            
            
            model_3d_rotation: (0.0, 0.0, 0.0),
            
            model_3d_offset: (0.0, 0.0, 0.0),
            
            fab_extent: None,
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for Cc3300modeniamozr {
    fn default() -> Self {
        Self::new()
    }
}
