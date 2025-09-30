use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    dma::{DmaRxBuf, DmaTxBuf},
    dma_buffers,
    gpio::{Level, Output, OutputConfig},
    i2c::master::{BusTimeout, Config as I2cConfig, I2c},
    spi::master::{Config as SpiConfig, Spi},
    spi::Mode,
    time::Rate,
    timer::{timg::TimerGroup, AnyTimer},
};

use crate::board::driver::{display::DisplayDriver, pps::PPSDriver, wifi_ble::WifiDriver};

#[allow(dead_code)]
pub struct Resources {
    pub(crate) display: DisplayDriver,
    pub(crate) wifi_ble: WifiDriver,
    pub(crate) pps: PPSDriver,
}

impl Resources {
    pub(crate) fn initialize() -> Self {
        let var_name = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
        let config = var_name;
        let peripherals = esp_hal::init(config);

        // esp_alloc::heap_allocator!(size: 12 * 1024);  // 11kB is max for the heap, otherwise "cannot move location counter backwards"
        esp_alloc::heap_allocator!(#[link_section = ".dram2_uninit"] size: 90000); // COEX needs more RAM - so we've added some more

        let timer0 = TimerGroup::new(peripherals.TIMG1);
        esp_hal_embassy::init(timer0.timer0);

        ////////////////////////// Display init ////////////////////////////
        let sclk = peripherals.GPIO18;
        let mosi = peripherals.GPIO23;
        let cs = peripherals.GPIO14;
        let bl = peripherals.GPIO32;

        let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(4092);
        let display_dma_channel = peripherals.DMA_SPI2;
        let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();
        let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();

        let spi = Spi::new(
            peripherals.SPI2,
            SpiConfig::default()
                .with_frequency(Rate::from_mhz(40))
                .with_mode(Mode::_0),
        )
        .unwrap()
        .with_sck(sclk)
        .with_mosi(mosi) // order matters
        .with_dma(display_dma_channel)
        .with_buffers(dma_rx_buf, dma_tx_buf)
        .into_async();

        let bl = Output::new(bl, Level::High, OutputConfig::default());
        let cs = Output::new(cs, Level::High, OutputConfig::default());
        let dc = Output::new(peripherals.GPIO27, Level::Low, OutputConfig::default());
        let rst = Output::new(peripherals.GPIO33, Level::Low, OutputConfig::default());
        let delay = Delay::new();
        let spi_device = ExclusiveDevice::new(spi, cs, delay).unwrap();
        let d = DisplayDriver::new(spi_device, bl, rst, dc);

        ////////////////////////// PPS Module init ////////////////////////////
        let i2c = I2c::new(
            peripherals.I2C0,
            I2cConfig::default()
                .with_frequency(Rate::from_khz(400))
                .with_timeout(BusTimeout::Maximum),
        )
        .unwrap()
        .with_sda(peripherals.GPIO21)
        .with_scl(peripherals.GPIO22)
        .into_async();
        let pps = PPSDriver::new(i2c, 0x35).expect("PPS module init failed");

        ////////////////////////// WiFi & BLE init ////////////////////////////
        let wifi_driver = crate::board::driver::wifi_ble::WifiDriver::new(
            peripherals.WIFI,
            peripherals.BT,
            AnyTimer::from(TimerGroup::new(peripherals.TIMG0).timer0),
            peripherals.RNG,
        );

        Self {
            display: d,
            wifi_ble: wifi_driver,
            pps,
        }
    }
}
