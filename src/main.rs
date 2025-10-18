#![no_std]
#![no_main]

extern crate alloc;

use {
    core::ptr::addr_of_mut,
    display_interface_spi::{SPIInterface, *},
    embedded_graphics::{
        mono_font::{ascii::{FONT_8X13, FONT_6X10}, MonoFont, MonoTextStyle},
        pixelcolor::{Rgb565, Bgr565, RgbColor},
        prelude::*,
        primitives::{Styled, Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, Triangle},
        text::{Alignment, Text},
    },
    embedded_text::{
        TextBox,
        style::{
            TextBoxStyleBuilder,
            HeightMode,
        },
        alignment::HorizontalAlignment as TextHorizontalAlignment,
    },
    embedded_layout::{
        layout::linear::LinearLayout,
        prelude::*,
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
    embassy_executor::Spawner,
    embassy_time::{Duration, Timer},
};

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASSWORD");

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

#[derive(Clone,Copy)]
enum StyleVariant {
    Header,
    Regular
}

impl<'spi> TFT<'spi> {
    const H_BG: Bgr565 = Bgr565::CSS_DARK_SLATE_GRAY;
    const H_S: Bgr565 = Bgr565::CYAN;
    const H_FG: Bgr565 = Bgr565::WHITE;

    const R_BG: Bgr565 = Bgr565::CSS_DARK_SLATE_GRAY;
    const R_S: Bgr565 = Bgr565::BLUE;
    const R_FG: Bgr565 = Bgr565::WHITE;

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

    pub fn clear(&mut self, color: Bgr565) {
        self.display.clear(candyflip(color)).unwrap();
    }

    pub fn part_clear(&mut self, x: i32, y: i32, w: u32, h: u32) {
        Rectangle::new(Point::new(x, y), Size::new(w, h))
            .into_styled(PrimitiveStyle::with_fill(Bgr565::BLACK))
            .draw(&mut self.draw_target())
            .unwrap();
    }

    pub fn containing_recty(&mut self, x: i32, y: i32, w: u32, h: u32, style: StyleVariant) -> Styled<Rectangle, PrimitiveStyle<Bgr565>> {
        let bg = match style {
            StyleVariant::Header => Self::H_BG,
            StyleVariant::Regular => Self::R_BG,
        };
        let s = match style {
            StyleVariant::Header => Self::H_S,
            StyleVariant::Regular => Self::R_S,
        };
        let style = PrimitiveStyleBuilder::new()
            .fill_color(bg)
            .stroke_color(s)
            .stroke_width(1)
            .build();
        Rectangle::new(Point::new(x, y), Size::new(w, h))
            .clone()
            .into_styled(style)
    }

    pub fn container(&mut self, margin: i32, y: i32, h: u32, style: StyleVariant) -> Styled<Rectangle, PrimitiveStyle<Bgr565>> {
        let width = self.display.bounding_box().size.width;
        self.containing_recty(margin, y, width - margin as u32, h, style)
    }

    pub fn println(&mut self, text: &str, x: i32, y: i32) {
        let style = MonoTextStyle::new(&FONT_6X10, Bgr565::WHITE);
        Text::with_alignment(text, Point::new(x, y), style, Alignment::Center)
            .draw(&mut self.draw_target())
            .unwrap();
    }
}

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}
/*
#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.capabilities());
    loop {
        if esp_wifi::wifi::wifi_state() == WifiState::StaConnected {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            Timer::after(Duration::from_millis(5000)).await
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.into(),
                password: PASSWORD.into(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start_async().await.unwrap();
            println!("Wifi started!");
        }
        println!("About to connect...");

        match controller.connect_async().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}*/


fn init_heap() {
    const HEAP_SIZE: usize = 64 * 1024;
    static mut HEAP: core::mem::MaybeUninit<[u8; HEAP_SIZE]> = core::mem::MaybeUninit::uninit();

    unsafe {
        esp_alloc::HEAP.add_region(esp_alloc::HeapRegion::new(
            addr_of_mut!(HEAP) as *mut u8,
            HEAP_SIZE,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }
}

struct Controller<'tft> {
    pub tft: TFT<'tft>,
}

impl Controller<'_> {
    async fn init(peripherals: Peripherals) -> Self {
        let tft = Self::init_screen(peripherals).await;
        let controller = Self {
            tft,
        };
        controller
    }

    async fn init_screen<'tft>(peripherals: Peripherals) -> TFT<'tft> {
        let dc = peripherals.GPIO9;
        let mosi = peripherals.GPIO6;
        let sclk = peripherals.GPIO4;
        let miso = peripherals.GPIO5;
        let cs = peripherals.GPIO7;
        let rst = peripherals.GPIO0;

        let mut tft = TFT::new(peripherals.SPI2, sclk, miso, mosi, cs, rst, dc);
        let _ = tft.draw_target().clear(Bgr565::BLACK);
        tft
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    init_heap();
    #[cfg(feature = "log")]
    {
        // The default log level can be specified here.
        // You can see the esp-println documentation： https://docs.rs/esp-println
        esp_println::logger::init_logger(log::LevelFilter::Info);
    }

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals: Peripherals = init(config);
/*    let timer1 = TimerGroup::new(peripherals.TIMG1);
    let rng = Rng::new();
    let esp_wifi_ctrl = &*mk_static!(
        EspWifiController<'static>,
        init(timer1.timer0, rng).unwrap()
    );
    let (controller, interfaces) = esp_wifi::wifi::new(esp_wifi_ctrl, peripherals.WIFI).unwrap();
    let wifi_interface = interfaces.sta;
    let seed = rng.random();
    let wifi_config = Config::dhcpv4(Default::default());
    let (stack, runner) = embassy_net::new(
        wifi_interface,
        wifi_config,
        mk_static!(StackResources<8>, StackResources::<8>::new()),
        seed,
    );*/

    let mut controller = Controller::init(peripherals).await;

    let display_area = controller.tft.display.bounding_box();
    let text_style = MonoTextStyle::new(&FONT_6X10, Bgr565::WHITE);
    let margin = 2;
    let display_width = display_area.size.width;
    let display_height = display_area.size.height;

    LinearLayout::vertical(
        Chain::new(
            LinearLayout::horizontal(
                Chain::new(
                    Text::new("Initializing~!", Point::new(margin, margin), text_style)
                ).append(
                    controller.tft.container(margin, -10, 20, StyleVariant::Header)
                )
            )
        )
    ).with_alignment(horizontal::Center)
    .arrange()
    .align_to(&display_area, horizontal::Center, vertical::Center)
    .draw(&mut controller.tft.draw_target())
    .unwrap();

    loop {
        // your business logic
    }
}
