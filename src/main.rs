#![no_std]
#![no_main]

use esp_backtrace as _;
use embedded_graphics::{
    mono_font::{
        ascii::FONT_8X13,
        MonoTextStyle,
        MonoFont,
    },
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{
        Line,
        PrimitiveStyle,
        PrimitiveStyleBuilder,
        Rectangle,
        Triangle,
    },
    text::{
        Alignment,
        Text,
    },
};
use display_interface_spi::{SPIInterface, *};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use esp_hal::{
    rng::Rng,
    timer::timg::TimerGroup,
    delay::Delay,
    gpio::{
        OutputPin, InputPin,
        AnyPin, Level, Input, Output, OutputConfig},
    peripherals::{ADC1, Peripherals, SPI2},
    spi::{
        master::{Config, Spi},
        Mode,
    },
    clock::CpuClock,
    time::Rate,
    Blocking,
    main,
    init,
};
use ili9341::{
    DisplaySize240x320,
    Ili9341,
    Orientation,
};
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

type TFTSpiDevice<'spi> = ExclusiveDevice<Spi<'spi, Blocking>, Output<'spi>, NoDelay>;
type TFTSpiInterface<'spi> =
SPIInterface<ExclusiveDevice<Spi<'spi, Blocking>, Output<'spi>, NoDelay>, Output<'spi>>;

pub struct TFT<'spi> {
    display: Ili9341<TFTSpiInterface<'spi>, Output<'spi>>,
}

impl<'spi> TFT<'spi> {
    pub fn new(
        spi2: SPI2<'spi>,
        sclk: impl OutputPin + 'spi,
        miso: impl InputPin + 'spi,
        mosi: impl OutputPin + 'spi,
        cs: impl OutputPin + 'spi,
        rst: impl OutputPin + 'spi,
        dc: impl OutputPin + 'spi,
    ) -> TFT<'spi> {
        let rst_output = Output::new(rst, Level::Low, OutputConfig::default());
        let dc_output = Output::new(dc, Level::Low, OutputConfig::default());
        let spi = Spi::new(spi2, Self::create_config())
            .unwrap()
            .with_sck(sclk)
            .with_miso(miso) // order matters
            .with_mosi(mosi) // order matters
            ;
        let cs_output = Output::new(cs, Level::High, OutputConfig::default());
        let spi_device = ExclusiveDevice::new_no_delay(spi, cs_output).unwrap();
        let interface = SPIInterface::new(spi_device, dc_output);

        let mut display = Ili9341::new(
            interface,
            rst_output,
            &mut Delay::new(),
            Orientation::Landscape,
            DisplaySize240x320,
        ).unwrap();

        TFT { display }
    }

    fn create_config() -> Config {
        Config::default()
            .with_frequency(Rate::from_mhz(10))
            .with_mode(Mode::_0)
    }

    pub fn clear(&mut self, color: Rgb565) {
        self.display.clear(color).unwrap();
    }

    pub fn part_clear(&mut self, x: i32, y: i32, w: u32, h: u32) {
        Rectangle::new(Point::new(x, y), Size::new(w, h))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
            .draw(&mut self.display)
            .unwrap();
    }

    pub fn println(&mut self, text: &str, x: i32, y: i32) {
        let style = MonoTextStyle::new(&FONT_8X13, Rgb565::RED);
        Text::with_alignment(text, Point::new(x, y), style, Alignment::Center)
            .draw(&mut self.display)
            .unwrap();
    }
}

#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals: Peripherals = init(config);
    esp_alloc::heap_allocator!(size: 72*1024);

    let dc = peripherals.GPIO9;
    let mosi = peripherals.GPIO6; // sdo -> MOSI
    let sclk = peripherals.GPIO4;
    let miso = peripherals.GPIO5; // sdi -> MISO
    let cs = peripherals.GPIO7;
    let rst = peripherals.GPIO0;

    let mut tft = TFT::new(peripherals.SPI2, sclk, miso, mosi, cs, rst, dc);
    tft.clear(Rgb565::WHITE);
    tft.println("nya~! -w-", 100, 40);

    loop {
        // your business logic
    }
}
