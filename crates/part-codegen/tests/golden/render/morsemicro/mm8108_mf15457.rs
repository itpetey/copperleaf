/// Morse Micro MM8108-MF15457 Wi-Fi HaLow module (MM8108 SoC).
///
/// Morse Micro Wi-Fi HaLow module
///
/// Datasheet: https://www.morsemicro.com/resources/datasheets/modules/MM8108-MF15457_Data_Sheet.pdf
///
/// # Pinout
///
/// | Pin | Name     | Purpose     | Notes                 |
/// |-----|----------|-------------|-----------------------|
/// | 1   | GND_8    | Ground      |                       |
/// | 2   | ANT      | Antenna     |                       |
/// | 3   | GND_3    | Ground      |                       |
/// | 4   | RESET_N  | Reset       | Pull low to reset. Asynchronous chip reset (active low). Supplied from VBAT domain. |
/// | 5   | WAKE     | Wake        | Leave open to disable power-saving. External wake from Deep Sleep and Snooze. Supplied from VBAT domain. |
/// | 6   | JTAG_TMS | JTAG        | alt: GPIO15           |
/// | 7   | JTAG_TCK | JTAG        | alt: GPIO13           |
/// | 8   | JTAG_TDO | JTAG        | alt: GPIO16           |
/// | 9   | JTAG_TDI | JTAG        | alt: GPIO14           |
/// | 10  | VBAT     | Supply      | 3.3 V VBAT supply     |
/// | 11  | GND_7    | Ground      |                       |
/// | 12  | SDIO_D0/SPI_MISO | SDIO/SPI    | alt: SPI_MISO. Pull up with 10 kΩ to 100 kΩ per SDIO spec. |
/// | 13  | SDIO_D3/SPI_CS | SDIO/SPI    | alt: SPI_CS. Pull up with 10 kΩ to 100 kΩ per SDIO spec. |
/// | 14  | SDIO_D1/SPI_INT | SDIO/SPI    | alt: SPI_INT. Pull up with 10 kΩ to 100 kΩ per SDIO spec. Level-triggered interrupt required for SPI mode. |
/// | 15  | SDIO_D2  | SDIO/SPI    | unused in SPI mode. Pull up with 10 kΩ to 100 kΩ per SDIO spec. |
/// | 16  | SDIO_CMD/SPI_MOSI | SDIO/SPI    | alt: SPI_MOSI. Pull up with 10 kΩ to 100 kΩ per SDIO spec. |
/// | 17  | SDIO_CLK/SPI_SCK | SDIO/SPI    | alt: SPI_SCK. SDIO 2.0 up to 50 MHz; SPI mode up to 80 MHz. |
/// | 18  | GPIO5    | GPIO        |                       |
/// | 19  | GPIO4    | GPIO        |                       |
/// | 20  | GND_6    | Ground      |                       |
/// | 21  | GPIO3    | GPIO        |                       |
/// | 22  | VDDIO    | Supply      | Host supply for digital I/O. Should be connected to the same power supply as the host MCU. |
/// | 23  | GND_5    | Ground      |                       |
/// | 24  | VBAT_TX  | Supply      | 3.3 V VBAT-TX supply  |
/// | 25  | VDD_USB  | Supply      | USB supply            |
/// | 26  | GND_4    | Ground      |                       |
/// | 27  | USB_D_N  | USB DM      | Floating in SPI mode  |
/// | 28  | USB_D_P  | USB DP      | Floating in SPI mode  |
/// | 29  | BUSY     | Busy        | BUSY signal output    |
/// | 30  | GND_2    | Ground      |                       |
/// | 31  | GPIO1    | GPIO        |                       |
/// | 32  | GPIO0    | GPIO        |                       |
/// | 33  | GPIO6    | GPIO        |                       |
/// | 34  | GPIO7    | GPIO        |                       |
/// | 35  | GPIO8    | GPIO        |                       |
/// | 36  | GPIO9    | GPIO        |                       |
/// | 37  | GPIO10   | GPIO        |                       |
/// | 38  | GND_1    | Ground      |                       |
pub struct Mm8108Mf15457 {
    pins: Vec<copperleaf::Pin>,
    mechanical: Vec<copperleaf::Pad>,
}

impl Mm8108Mf15457 {
    pub const GND_8: copperleaf::PinRef = copperleaf::PinRef("GND_8");
    pub const ANT: copperleaf::PinRef = copperleaf::PinRef("ANT");
    pub const GND_3: copperleaf::PinRef = copperleaf::PinRef("GND_3");
    pub const RESET_N: copperleaf::PinRef = copperleaf::PinRef("RESET_N");
    pub const WAKE: copperleaf::PinRef = copperleaf::PinRef("WAKE");
    pub const JTAG_TMS: copperleaf::PinRef = copperleaf::PinRef("JTAG_TMS");
    pub const JTAG_TCK: copperleaf::PinRef = copperleaf::PinRef("JTAG_TCK");
    pub const JTAG_TDO: copperleaf::PinRef = copperleaf::PinRef("JTAG_TDO");
    pub const JTAG_TDI: copperleaf::PinRef = copperleaf::PinRef("JTAG_TDI");
    pub const VBAT: copperleaf::PinRef = copperleaf::PinRef("VBAT");
    pub const GND_7: copperleaf::PinRef = copperleaf::PinRef("GND_7");
    pub const SDIO_D0_SPI_MISO: copperleaf::PinRef = copperleaf::PinRef("SDIO_D0/SPI_MISO");
    pub const SDIO_D3_SPI_CS: copperleaf::PinRef = copperleaf::PinRef("SDIO_D3/SPI_CS");
    pub const SDIO_D1_SPI_INT: copperleaf::PinRef = copperleaf::PinRef("SDIO_D1/SPI_INT");
    pub const SDIO_D2: copperleaf::PinRef = copperleaf::PinRef("SDIO_D2");
    pub const SDIO_CMD_SPI_MOSI: copperleaf::PinRef = copperleaf::PinRef("SDIO_CMD/SPI_MOSI");
    pub const SDIO_CLK_SPI_SCK: copperleaf::PinRef = copperleaf::PinRef("SDIO_CLK/SPI_SCK");
    pub const GPIO5: copperleaf::PinRef = copperleaf::PinRef("GPIO5");
    pub const GPIO4: copperleaf::PinRef = copperleaf::PinRef("GPIO4");
    pub const GND_6: copperleaf::PinRef = copperleaf::PinRef("GND_6");
    pub const GPIO3: copperleaf::PinRef = copperleaf::PinRef("GPIO3");
    pub const VDDIO: copperleaf::PinRef = copperleaf::PinRef("VDDIO");
    pub const GND_5: copperleaf::PinRef = copperleaf::PinRef("GND_5");
    pub const VBAT_TX: copperleaf::PinRef = copperleaf::PinRef("VBAT_TX");
    pub const VDD_USB: copperleaf::PinRef = copperleaf::PinRef("VDD_USB");
    pub const GND_4: copperleaf::PinRef = copperleaf::PinRef("GND_4");
    pub const USB_D_N: copperleaf::PinRef = copperleaf::PinRef("USB_D_N");
    pub const USB_D_P: copperleaf::PinRef = copperleaf::PinRef("USB_D_P");
    pub const BUSY: copperleaf::PinRef = copperleaf::PinRef("BUSY");
    pub const GND_2: copperleaf::PinRef = copperleaf::PinRef("GND_2");
    pub const GPIO1: copperleaf::PinRef = copperleaf::PinRef("GPIO1");
    pub const GPIO0: copperleaf::PinRef = copperleaf::PinRef("GPIO0");
    pub const GPIO6: copperleaf::PinRef = copperleaf::PinRef("GPIO6");
    pub const GPIO7: copperleaf::PinRef = copperleaf::PinRef("GPIO7");
    pub const GPIO8: copperleaf::PinRef = copperleaf::PinRef("GPIO8");
    pub const GPIO9: copperleaf::PinRef = copperleaf::PinRef("GPIO9");
    pub const GPIO10: copperleaf::PinRef = copperleaf::PinRef("GPIO10");
    pub const GND_1: copperleaf::PinRef = copperleaf::PinRef("GND_1");

    pub fn new() -> Self {
        use copperleaf::{Pin, PowerSpec, Role, units::UnitExt};

        Self {
            pins: vec![
                Pin::build("GND_8").number("1").pos(0.0, 0.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("ANT").number("2").pos(1.0, 0.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").role(Role::AnalogIn).rf_limits().pin(),
                Pin::build("GND_3").number("3").pos(2.0, 0.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("RESET_N").number("4").pos(3.0, 0.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("WAKE").number("5").pos(4.0, 0.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("JTAG_TMS").number("6").pos(5.0, 0.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("JTAG_TCK").number("7").pos(6.0, 0.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").clk(1.0),
                Pin::build("JTAG_TDO").number("8").pos(7.0, 0.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("JTAG_TDI").number("9").pos(8.0, 0.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("VBAT").number("10").pos(9.0, 0.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(3.0.volt(), 3.6.volt(), 0.3.amp()).pin(),
                Pin::build("GND_7").number("11").pos(10.0, 0.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("SDIO_D0/SPI_MISO").number("12").pos(10.0, 1.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").spi(80.0),
                Pin::build("SDIO_D3/SPI_CS").number("13").pos(10.0, 2.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").spi(80.0),
                Pin::build("SDIO_D1/SPI_INT").number("14").pos(10.0, 3.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").spi(80.0),
                Pin::build("SDIO_D2").number("15").pos(10.0, 4.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("SDIO_CMD/SPI_MOSI").number("16").pos(10.0, 5.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").spi(80.0),
                Pin::build("SDIO_CLK/SPI_SCK").number("17").pos(10.0, 6.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").clk(80.0),
                Pin::build("GPIO5").number("18").pos(10.0, 7.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO4").number("19").pos(10.0, 8.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GND_6").number("20").pos(10.0, 9.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("GPIO3").number("21").pos(9.0, 9.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("VDDIO").number("22").pos(8.0, 9.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(2.25.volt(), 3.6.volt(), 0.05.amp()).pin(),
                Pin::build("GND_5").number("23").pos(7.0, 9.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("VBAT_TX").number("24").pos(6.0, 9.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(3.0.volt(), 3.6.volt(), 0.5.amp()).pin(),
                Pin::build("VDD_USB").number("25").pos(5.0, 9.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").pwr(3.0.volt(), 3.6.volt(), 0.1.amp()).pin(),
                Pin::build("GND_4").number("26").pos(4.0, 9.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("USB_D_N").number("27").pos(3.0, 9.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("USB_D_P").number("28").pos(2.0, 9.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("BUSY").number("29").pos(1.0, 9.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GND_2").number("30").pos(0.0, 9.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
                Pin::build("GPIO1").number("31").pos(0.0, 8.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO0").number("32").pos(0.0, 7.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO6").number("33").pos(0.0, 6.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO7").number("34").pos(0.0, 5.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO8").number("35").pos(0.0, 4.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO9").number("36").pos(0.0, 3.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GPIO10").number("37").pos(0.0, 2.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").dio(),
                Pin::build("GND_1").number("38").pos(0.0, 1.0).rotation(0.0).length(0.6).width(0.6).height(0.6).pad_type("smd").pad_shape("rect").solder_mask_margin(0.102).layers("F.Cu F.Mask F.Paste").gnd(),
            ],
            mechanical: vec![
            ],
        }
    }

    pub fn constraints(&self) -> Vec<copperleaf::Constraint> {
        use copperleaf::{Constraint, units::UnitExt};
        vec![
            Constraint::Decoupling { values: vec![100.0.nf(), 10.0.uf()], per_pin: false },
            Constraint::LengthMatch { group: "SPI0_BUS".into(), skew_ps: 200.0 },
            Constraint::MaxJunction { temp: 85.0.celsius() },
        ]
    }
}

impl copperleaf::Component for Mm8108Mf15457 {
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
            symbol: Some("MM8108-MF15457".into()),
            
            footprint: Some("MM8108-MF15457".into()),
            
            datasheet: Some("https://www.morsemicro.com/resources/datasheets/modules/MM8108-MF15457_Data_Sheet.pdf".into()),
            
            description: Some("Morse Micro Wi-Fi HaLow module".into()),
            
            
            model_3d: None,
            model_3d_data: Some("<elided:9068144:6cc8bea1c4ef70d6>".into()),
            
            model_3d_rotation: (-90.0, 0.0, 0.0),
            
            model_3d_offset: (-5.0, 4.5, 0.0),
            
            
            fab_extent: None,
            capacitance: None,
            is_bypass: false,
        })
    }
}

impl Default for Mm8108Mf15457 {
    fn default() -> Self {
        Self::new()
    }
}
