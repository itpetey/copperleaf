/// Texas Instruments CC3300ENJARSBR SimpleLink Wi-Fi 6 Companion IC (WQFN-40)
///
/// Datasheet: https://www.ti.com/lit/gpn/cc3300
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | PA_LDO_OUT | LDO output  | RF power amplifier LDO output (analog). Output rail VDDPA_OUT. VPA peak current can reach 340mA during device calibration. |
/// | 2   | RF_BG    | RF I/O      | WLAN 2.4GHz RF port (RF I/O). WLAN operational frequency 2412-2472 MHz. |
/// | 3   | GND      | Ground      | Ground.               |
/// | 4   | VDDA_IN1 | Supply      | 1.8V supply for analog domain (VMAIN rail). |
/// | 5   | VDDA_IN2 | Supply      | 1.8V supply for analog domain (VMAIN rail). |
/// | 6   | HFXT_P   | Crystal     | 40MHz crystal fast clock, positive terminal (XTAL_P). See fast clock XTAL specifications. |
/// | 7   | HFXT_M   | Crystal     | 40MHz crystal fast clock, negative terminal (XTAL_N). See fast clock XTAL specifications. |
/// | 8   | COEX_GRANT2 | I/O         | External coexistence interface - grant (output, 3-wire or 1-wire PTA). |
/// | 9   | COEX_PRIORITY2 | I/O         | External coexistence interface - priority (input, 3-wire or 1-wire PTA). |
/// | 10  | COEX_REQ2 | I/O         | External coexistence interface - request (input, 3-wire or 1-wire PTA). |
/// | 11  | UART RTS | No connect  | No connect on CC3300 (BLE HCI UART RTS on CC3301). Leave unconnected. |
/// | 12  | UART CTS | No connect  | No connect on CC3300 (BLE HCI UART CTS on CC3301). Leave unconnected. |
/// | 13  | UART RX  | No connect  | No connect on CC3300 (BLE HCI UART RX on CC3301). Leave unconnected. |
/// | 14  | UART TX  | No connect  | No connect on CC3300 (BLE HCI UART TX on CC3301). Leave unconnected. |
/// | 15  | ANT_SEL2 | I/O         | Antenna select control line (output). Used for optional antenna diversity or selection. |
/// | 16  | GND_1    | Ground      | Ground.               |
/// | 17  | VIO      | Supply      | 1.8V IO supply (VIO rail). |
/// | 18  | SDIO CMD | I/O         | SDIO command or SPI PICO (digital I/O). |
/// | 19  | SDIO CLK | Clock       | SDIO clock or SPI clock (input). SDIO supports up to 52MHz, SPI up to 26MHz. |
/// | 20  | GND_2    | Ground      | Ground.               |
/// | 21  | SDIO D3  | I/O         | SDIO data D3 or SPI CS (digital I/O). |
/// | 22  | SDIO D2  | I/O         | SDIO data D2 (digital I/O). |
/// | 23  | SDIO D1  | I/O         | SDIO data D1 (digital I/O). |
/// | 24  | SDIO D0  | I/O         | SDIO data D0 or SPI POCI (digital I/O). |
/// | 25  | GND_3    | Ground      | Ground.               |
/// | 26  | SWCLK    | Clock       | Serial wire debug clock (input). |
/// | 27  | SWDIO    | I/O         | Serial wire debug I/O. |
/// | 28  | LOGGER3  | I/O         | Tracer (UART TX debug logger, output). Pin state sensed by device during boot. |
/// | 29  | HOST_IRQ_WL3 | I/O         | Interrupt request to host for WLAN (output). Pin state sensed by device during boot. |
/// | 30  | HOST_IRQ_BLE3 | No connect  | No connect on CC3300 (BLE host interrupt on CC3301). Leave unconnected. |
/// | 31  | DIG_LDO_OUT | LDO output  | Digital LDO output to decoupling capacitor (analog). |
/// | 32  | VDD_MAIN_IN | Supply      | 1.8V supply input for SRAM and digital (VMAIN rail). |
/// | 33  | NRESET   | I/O         | Reset line for enabling or disabling device, active low. Hold low for at least 10us after external supplies are stable. |
/// | 34  | SLOW_CLK_IN | Clock       | 32.768kHz slow/RTC clock input. Leave unconnected to use the internal slow clock. |
/// | 35  | VPP_IN   | Supply      | 1.8V OTP programming input supply (VPP rail). |
/// | 36  | FAST_CLK_REQ | I/O         | Fast clock request from the device (output). |
/// | 37  | GND_4    | Ground      | Ground.               |
/// | 38  | GND_5    | Ground      | Ground.               |
/// | 39  | PA_LDO_IN | Supply      | 3.3V supply for PA (VPA rail). Peak current can reach 340mA during device calibration. |
/// | 40  | PA_LDO_IN_1 | Supply      | 3.3V supply for PA (VPA rail). Peak current can reach 340mA during device calibration. |
/// | 41  | EPAD     | Supply      | Exposed thermal pad. Must be soldered to the PCB for thermal and mechanical performance. |
pub struct Cc3300enjarsbr {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Cc3300enjarsbr {
    pub const PA_LDO_OUT: copperleaf::PinRef = copperleaf::PinRef("PA_LDO_OUT");
    pub const RF_BG: copperleaf::PinRef = copperleaf::PinRef("RF_BG");
    pub const GND: copperleaf::PinRef = copperleaf::PinRef("GND");
    pub const VDDA_IN1: copperleaf::PinRef = copperleaf::PinRef("VDDA_IN1");
    pub const VDDA_IN2: copperleaf::PinRef = copperleaf::PinRef("VDDA_IN2");
    pub const HFXT_P: copperleaf::PinRef = copperleaf::PinRef("HFXT_P");
    pub const HFXT_M: copperleaf::PinRef = copperleaf::PinRef("HFXT_M");
    pub const COEX_GRANT2: copperleaf::PinRef = copperleaf::PinRef("COEX_GRANT2");
    pub const COEX_PRIORITY2: copperleaf::PinRef = copperleaf::PinRef("COEX_PRIORITY2");
    pub const COEX_REQ2: copperleaf::PinRef = copperleaf::PinRef("COEX_REQ2");
    pub const UART_RTS: copperleaf::PinRef = copperleaf::PinRef("UART RTS");
    pub const UART_CTS: copperleaf::PinRef = copperleaf::PinRef("UART CTS");
    pub const UART_RX: copperleaf::PinRef = copperleaf::PinRef("UART RX");
    pub const UART_TX: copperleaf::PinRef = copperleaf::PinRef("UART TX");
    pub const ANT_SEL2: copperleaf::PinRef = copperleaf::PinRef("ANT_SEL2");
    pub const GND_1: copperleaf::PinRef = copperleaf::PinRef("GND_1");
    pub const VIO: copperleaf::PinRef = copperleaf::PinRef("VIO");
    pub const SDIO_CMD: copperleaf::PinRef = copperleaf::PinRef("SDIO CMD");
    pub const SDIO_CLK: copperleaf::PinRef = copperleaf::PinRef("SDIO CLK");
    pub const GND_2: copperleaf::PinRef = copperleaf::PinRef("GND_2");
    pub const SDIO_D3: copperleaf::PinRef = copperleaf::PinRef("SDIO D3");
    pub const SDIO_D2: copperleaf::PinRef = copperleaf::PinRef("SDIO D2");
    pub const SDIO_D1: copperleaf::PinRef = copperleaf::PinRef("SDIO D1");
    pub const SDIO_D0: copperleaf::PinRef = copperleaf::PinRef("SDIO D0");
    pub const GND_3: copperleaf::PinRef = copperleaf::PinRef("GND_3");
    pub const SWCLK: copperleaf::PinRef = copperleaf::PinRef("SWCLK");
    pub const SWDIO: copperleaf::PinRef = copperleaf::PinRef("SWDIO");
    pub const LOGGER3: copperleaf::PinRef = copperleaf::PinRef("LOGGER3");
    pub const HOST_IRQ_WL3: copperleaf::PinRef = copperleaf::PinRef("HOST_IRQ_WL3");
    pub const HOST_IRQ_BLE3: copperleaf::PinRef = copperleaf::PinRef("HOST_IRQ_BLE3");
    pub const DIG_LDO_OUT: copperleaf::PinRef = copperleaf::PinRef("DIG_LDO_OUT");
    pub const VDD_MAIN_IN: copperleaf::PinRef = copperleaf::PinRef("VDD_MAIN_IN");
    pub const NRESET: copperleaf::PinRef = copperleaf::PinRef("NRESET");
    pub const SLOW_CLK_IN: copperleaf::PinRef = copperleaf::PinRef("SLOW_CLK_IN");
    pub const VPP_IN: copperleaf::PinRef = copperleaf::PinRef("VPP_IN");
    pub const FAST_CLK_REQ: copperleaf::PinRef = copperleaf::PinRef("FAST_CLK_REQ");
    pub const GND_4: copperleaf::PinRef = copperleaf::PinRef("GND_4");
    pub const GND_5: copperleaf::PinRef = copperleaf::PinRef("GND_5");
    pub const PA_LDO_IN: copperleaf::PinRef = copperleaf::PinRef("PA_LDO_IN");
    pub const PA_LDO_IN_1: copperleaf::PinRef = copperleaf::PinRef("PA_LDO_IN_1");
    pub const EPAD: copperleaf::PinRef = copperleaf::PinRef("EPAD");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("PA_LDO_OUT").number("1").pos(-2.5273, -1.800225).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").role(Role::PowerOut).power_spec(PowerSpec { v_min: 1.9.volt(), v_max: 1.9.volt(), v_nom: Some(1.9.volt()), i_max: 0.34.amp() }).pin(),
                Pin::build("RF_BG").number("2").pos(-2.5273, -1.400175).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").role(Role::AnalogIn).rf_limits().pin(),
                Pin::build("GND").number("3").pos(-2.5273, -1.000125).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("VDDA_IN1").number("4").pos(-2.5273, -0.600075).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").pwr(1.62.volt(), 1.98.volt(), 0.05.amp()).pin(),
                Pin::build("VDDA_IN2").number("5").pos(-2.5273, -0.200025).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").pwr(1.62.volt(), 1.98.volt(), 0.05.amp()).pin(),
                Pin::build("HFXT_P").number("6").pos(-2.5273, 0.200025).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").analog_in(),
                Pin::build("HFXT_M").number("7").pos(-2.5273, 0.600075).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").analog_in(),
                Pin::build("COEX_GRANT2").number("8").pos(-2.5273, 1.000125).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("COEX_PRIORITY2").number("9").pos(-2.5273, 1.400175).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("COEX_REQ2").number("10").pos(-2.5273, 1.800225).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("UART RTS").number("11").pos(-1.800225, 2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("UART CTS").number("12").pos(-1.400175, 2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("UART RX").number("13").pos(-1.000125, 2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("UART TX").number("14").pos(-0.600075, 2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("ANT_SEL2").number("15").pos(-0.200025, 2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("GND_1").number("16").pos(0.200025, 2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("VIO").number("17").pos(0.600075, 2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").pwr(1.62.volt(), 1.98.volt(), 0.02.amp()).pin(),
                Pin::build("SDIO CMD").number("18").pos(1.000125, 2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("SDIO CLK").number("19").pos(1.400175, 2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").clk(52.0),
                Pin::build("GND_2").number("20").pos(1.800225, 2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("SDIO D3").number("21").pos(2.5273, 1.800225).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("SDIO D2").number("22").pos(2.5273, 1.400175).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("SDIO D1").number("23").pos(2.5273, 1.000125).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("SDIO D0").number("24").pos(2.5273, 0.600075).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("GND_3").number("25").pos(2.5273, 0.200025).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("SWCLK").number("26").pos(2.5273, -0.200025).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").clk(10.0),
                Pin::build("SWDIO").number("27").pos(2.5273, -0.600075).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("LOGGER3").number("28").pos(2.5273, -1.000125).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("HOST_IRQ_WL3").number("29").pos(2.5273, -1.400175).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("HOST_IRQ_BLE3").number("30").pos(2.5273, -1.800225).rotation(90.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("DIG_LDO_OUT").number("31").pos(1.800225, -2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").role(Role::PowerOut).power_spec(PowerSpec { v_min: 1.1.volt(), v_max: 1.1.volt(), v_nom: Some(1.1.volt()), i_max: 0.02.amp() }).pin(),
                Pin::build("VDD_MAIN_IN").number("32").pos(1.400175, -2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").pwr(1.62.volt(), 1.98.volt(), 0.25.amp()).pin(),
                Pin::build("NRESET").number("33").pos(1.000125, -2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("SLOW_CLK_IN").number("34").pos(0.600075, -2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").clk(0.032768),
                Pin::build("VPP_IN").number("35").pos(0.200025, -2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").pwr(1.62.volt(), 1.98.volt(), 0.01.amp()).pin(),
                Pin::build("FAST_CLK_REQ").number("36").pos(-0.200025, -2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").dio(),
                Pin::build("GND_4").number("37").pos(-0.600075, -2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("GND_5").number("38").pos(-1.000125, -2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").gnd(),
                Pin::build("PA_LDO_IN").number("39").pos(-1.400175, -2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").pwr(3.0.volt(), 3.6.volt(), 0.29.amp()).pin(),
                Pin::build("PA_LDO_IN_1").number("40").pos(-1.800225, -2.5273).rotation(0.0).length(0.9144).width(0.2032).height(0.9144).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").pwr(3.0.volt(), 3.6.volt(), 0.29.amp()).pin(),
                Pin::build("EPAD").number("41").pos(0.0, 0.0).rotation(0.0).length(3.6068).width(3.6068).height(3.6068).pad_type("smd").pad_shape("rect").layers("F.Cu F.Paste F.Mask").pwr_fixed(0.0.volt(), 0.5.amp()).pin(),
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

impl copperleaf::Component for Cc3300enjarsbr {
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
            symbol: Some("CC3300ENJARSBR".into()),
            
            footprint: Some("CC3300ENJARSBR".into()),
            
            datasheet: Some("https://www.ti.com/lit/gpn/cc3300".into()),
            
            
            description: None,
            
            model_3d: None,
            model_3d_data: Some("<elided:1494388:037e7a3cf95c3521>".into()),
            
            
            model_3d_rotation: (0.0, 0.0, 0.0),
            
            model_3d_offset: (0.0, 0.0, 0.0),
            
            fab_extent: None,
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for Cc3300enjarsbr {
    fn default() -> Self {
        Self::new()
    }
}
