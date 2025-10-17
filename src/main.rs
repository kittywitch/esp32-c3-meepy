#![no_std]
#![no_main]

use {
    display_interface_spi::{SPIInterface, *},
    embedded_graphics::{
        mono_font::{ascii::FONT_8X13, MonoFont, MonoTextStyle},
        pixelcolor::{Rgb565, Bgr565, RgbColor},
        prelude::*,
        primitives::{Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, Triangle},
        text::{Alignment, Text},
    },
    embedded_hal_bus::spi::{ExclusiveDevice, NoDelay},
    esp_backtrace as _,
    esp_hal::{
        clock::CpuClock,
        delay::Delay,
        gpio::{AnyPin, Input, InputPin, Level, Output, OutputConfig, OutputPin},
        init, main,
        peripherals::{Peripherals, ADC1, SPI2},
        rng::Rng,
        spi::{
            master::{Config, Spi},
            Mode,
        },
        time::Rate,
        timer::timg::TimerGroup,
        Blocking,
    },
    esp_println::println,
    ili9341::{DisplaySize240x320, Ili9341, Orientation},
};

esp_bootloader_esp_idf::esp_app_desc!();

type TFTSpiDevice<'spi> = ExclusiveDevice<Spi<'spi, Blocking>, Output<'spi>, NoDelay>;
type TFTSpiInterface<'spi> =
    SPIInterface<ExclusiveDevice<Spi<'spi, Blocking>, Output<'spi>, NoDelay>, Output<'spi>>;

type Ili<'spi> = Ili9341<TFTSpiInterface<'spi>, Output<'spi>>;

pub struct TFT<'spi> {
    display: Ili<'spi>,
}

impl<'spi> TFT<'spi> {
    fn draw_target(&mut self) -> DrawFlipper<'_, 'spi> {
        DrawFlipper {
            display: &mut self.display,
        }
    }
}

fn candyflip(color: Bgr565) -> Rgb565 {
    unsafe {
        core::mem::transmute::<Bgr565, Rgb565>(color)
    }
}
fn flipcandy(color: Rgb565) -> Bgr565 {
    unsafe {
        core::mem::transmute::<Rgb565, Bgr565>(color)
    }
}

struct DrawFlipper<'a, 'spi> {
    display: &'a mut Ili<'spi>,
}

impl<'a, 'spi> DrawTarget for DrawFlipper<'a, 'spi> {
    type Error = <Ili<'spi> as DrawTarget>::Error;
    type Color = Bgr565;//<Ili<'spi> as DrawTarget>::Color;
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let width = self.bounding_box().size.width as i32;
        self.display.draw_iter(pixels.into_iter().map(|p| {
            Pixel(
                Point {
                    x: width - p.0.x - 1,
                    y: p.0.y,
                },
                candyflip(p.1),
            )
        }))
    }
    fn fill_contiguous<I>(
        &mut self,
        area: &Rectangle,
        colors: I,
    ) -> Result<(), Self::Error>
       where I: IntoIterator<Item = Self::Color> {
        self.display.fill_contiguous(area, colors.into_iter().map(|c| candyflip(c)))
    }
    fn fill_solid(
        &mut self,
        area: &Rectangle,
        color: Self::Color,
    ) -> Result<(), Self::Error> {
        self.display.fill_solid(area, candyflip(color))
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.display.clear(candyflip(color))
    }
}

impl<'a> Dimensions for DrawFlipper<'a, '_> {
    fn bounding_box(&self) -> Rectangle {
        self.display.bounding_box()
    }
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
        )
        .unwrap();

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
            .into_styled(PrimitiveStyle::with_fill(Bgr565::BLACK))
            .draw(&mut self.draw_target())
            .unwrap();
    }

    pub fn println(&mut self, text: &str, x: i32, y: i32) {
        let style = MonoTextStyle::new(&FONT_8X13, Bgr565::WHITE);
        Text::with_alignment(text, Point::new(x, y), style, Alignment::Center)
            .draw(&mut self.draw_target())
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
    tft.draw_target().clear(Bgr565::BLACK);
    tft.println("nya~! -w-", 100, 40);

    loop {
        // your business logic
    }
}
