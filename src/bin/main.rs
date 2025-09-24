#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

extern crate alloc;

use alloc::boxed::Box;
use defmt;
use esp_backtrace as _;
use esp_println as _;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, Primitive, PrimitiveStyle, Triangle},
};

use esp_hal::clock::CpuClock;
use esp_hal::gpio::OutputConfig;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{delay::Delay, gpio::{Level, Output}, spi::{
    master::{Config, Spi},
    Mode,
}, time::Rate};
use esp_wifi::ble::controller::BleConnector;

use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::interface::SpiInterface;
use mipidsi::{models::ILI9342CRgb565, Builder}; // Provides the builder for Display // Provides the required color type

use altreg_fire27_rs::driver::display::Display as DisplayType;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[no_mangle]
pub extern "Rust" fn _esp_println_timestamp() -> u64 {
    esp_hal::time::Instant::now()
        .duration_since_epoch()
        .as_millis()
}


#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.5.0

    // defmt-espflash will be used for logging; no esp_println logger initialization needed

    let var_name = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let config = var_name;
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(#[unsafe(link_section = ".dram2_uninit")] size: 64 * 1024);

    let timer0 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);

    let mut delay = Delay::new();
    let mut buffer: &'static mut [u8] = Box::leak(Box::new([0_u8; 512]));
    let sclk = peripherals.GPIO18;
    let mosi = peripherals.GPIO23;
    let cs = peripherals.GPIO14;
    let bl = peripherals.GPIO32;
    
    
    let spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_khz(40000))
            .with_mode(Mode::_0),
    )
        .unwrap()
        .with_sck(sclk)
        .with_mosi(mosi) // order matters
        .into_async();

    let bl_output = Output::new(bl, Level::High, OutputConfig::default());
    let cs_output = Output::new(cs, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO27, Level::Low, OutputConfig::default());
    let mut rst = Output::new(peripherals.GPIO33, Level::Low, OutputConfig::default());
    rst.set_high();

    let mut delay = Delay::new();
    let spi_device = ExclusiveDevice::new(spi, cs_output, delay).unwrap();
    let di = SpiInterface::new(spi_device, dc, buffer);

    let mut d = DisplayType::new(di, bl_output, rst);

        //
        //
        // // Define the display from the display interface and initialize it
        // let mut display = Builder::new(ILI9342CRgb565, di)
        //     .invert_colors(mipidsi::options::ColorInversion::Inverted)
        //     .color_order(mipidsi::options::ColorOrder::Bgr)
        //     .display_size(320, 240)
        //     .reset_pin(rst)
        //     .init(&mut delay)
        //     .unwrap();
        //
        // Make the display all black
        d.display.clear(Rgb565::BLACK).unwrap();

        // Draw a smiley face with white eyes and a red mouth
        draw_smiley(d.display).unwrap();

    defmt::info!("Embassy initialized!");


    let rng = esp_hal::rng::Rng::new(peripherals.RNG);
    let timer1 = TimerGroup::new(peripherals.TIMG0);
    let wifi_init = Box::leak(Box::new(
        esp_wifi::init(timer1.timer0, rng).expect("Failed to initialize WIFI/BLE controller"),
    ));
    let (mut _wifi_controller, _interfaces) = esp_wifi::wifi::new(&wifi_init, peripherals.WIFI)
        .expect("Failed to initialize WIFI controller");
    // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    let transport = BleConnector::new(wifi_init, peripherals.BT);

    // Spawn ble_scan::run in a separate Embassy task
    spawner.spawn(ble_scan_task(transport)).unwrap();

    loop {
        defmt::info!("Hello world!");
        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-rc.0/examples/src/bin
}


#[embassy_executor::task]
async fn ble_scan_task(transport: esp_wifi::ble::controller::BleConnector<'static>) {
    let controller = bt_hci::controller::ExternalController::<_, 20>::new(transport);
    altreg_fire27_rs::ble_scan::run(controller).await;
}

fn draw_smiley<T: DrawTarget<Color = Rgb565>>(display: &mut T) -> Result<(), T::Error> {
    // Draw the left eye as a circle located at (50, 100), with a diameter of 40, filled with white
    Circle::new(Point::new(50, 100), 40)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
        .draw(display)?;

    // Draw the right eye as a circle located at (50, 200), with a diameter of 40, filled with white
    Circle::new(Point::new(50, 200), 40)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
        .draw(display)?;

    // Draw an upside down red triangle to represent a smiling mouth
    Triangle::new(
        Point::new(130, 140),
        Point::new(130, 200),
        Point::new(160, 170),
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
    .draw(display)?;

    // Cover the top part of the mouth with a black triangle so it looks closed instead of open
    Triangle::new(
        Point::new(130, 150),
        Point::new(130, 190),
        Point::new(150, 170),
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
    .draw(display)?;

    Ok(())
}
