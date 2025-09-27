use crate::board::driver::display::{DisplayDriver};
use alloc::boxed::Box;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{clock::CpuClock, delay::Delay, dma_buffers, gpio::{Level, Output, OutputConfig}, spi::master::{Config, Spi}, spi::Mode, time::Rate, timer::{timg::TimerGroup, AnyTimer}};
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use mipidsi::interface::SpiInterface;

pub struct Resources {
    pub(crate) display: DisplayDriver,
    pub(crate) wifi_ble: crate::board::driver::wifi_ble::WifiDriver,
}

impl Resources {
    pub(crate) fn initialize() -> Self {
        let var_name = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
        let config = var_name;
        let peripherals = esp_hal::init(config);

        // esp_alloc::heap_allocator!(size: 12 * 1024);  // 11kB is max for the heap, otherwise "cannot move location counter backwards"
        esp_alloc::heap_allocator!(#[link_section = ".dram2_uninit"] size: 64000); // COEX needs more RAM - so we've added some more

        let timer0 = TimerGroup::new(peripherals.TIMG1);
        esp_hal_embassy::init(timer0.timer0);

        let buffer: &'static mut [u8] = Box::leak(Box::new([0_u8; 512]));
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
            Config::default()
                .with_frequency(Rate::from_mhz(40))
                .with_mode(Mode::_0),
        )
        .unwrap()
        .with_sck(sclk)
        .with_mosi(mosi) // order matters
        .with_dma(display_dma_channel)
        .with_buffers(dma_rx_buf, dma_tx_buf)
        .into_async();

        let bl_output = Output::new(bl, Level::High, OutputConfig::default());
        let cs_output = Output::new(cs, Level::High, OutputConfig::default());
        let dc = Output::new(peripherals.GPIO27, Level::Low, OutputConfig::default());
        let mut rst = Output::new(peripherals.GPIO33, Level::Low, OutputConfig::default());
        rst.set_high();

        let delay = Delay::new();
        let spi_device = ExclusiveDevice::new(spi, cs_output, delay).unwrap();
        let di = SpiInterface::new(spi_device, dc, buffer);

        let d = DisplayDriver::new(di, bl_output, rst);

        let wifi_driver = crate::board::driver::wifi_ble::WifiDriver::new(
            peripherals.WIFI,
            peripherals.BT,
            AnyTimer::from(TimerGroup::new(peripherals.TIMG0).timer0),
            peripherals.RNG,
        );

        Self {
            display: d,
            wifi_ble: wifi_driver,
        }
    }
}
