//! Display and sd card are tightly coupled by sharing the same SPI bus.
//! This module therefore handles both in a common task to avoid unnecessary low-level/high-frequency synchronization.

use embedded_hal_bus::spi::RefCellDevice;
use embedded_sdmmc::SdCard;
use esp_hal::{
    delay::Delay,
    dma::AnySpiDmaChannel,
    gpio::{AnyPin, Output},
    spi::master::{AnySpi, SpiDmaBus},
    Async,
};

use crate::board::driver::display::DisplayDriver;
use crate::StartupError;

type SpiBusType = SpiDmaBus<'static, Async>;
pub type SpiDeviceType<'a> = RefCellDevice<'a, SpiBusType, Output<'static>, Delay>;
pub type SdCardType = SdCard<SpiDeviceType<'static>, Delay>;

pub struct Spi2Resources<'a> {
    pub spi2: AnySpi<'a>,
    pub sck: AnyPin<'a>,
    pub mosi: AnyPin<'a>,
    pub miso: AnyPin<'a>,
    pub card_cs: AnyPin<'a>,
    pub display_dma: AnySpiDmaChannel<'a>,
    pub display_cs: AnyPin<'a>,
    pub display_bl: AnyPin<'a>,
    pub display_dc: AnyPin<'a>,
    pub display_rst: AnyPin<'a>,
}

impl Spi2Resources<'static> {
    pub fn into_devices(self) -> Result<(SdCardType, DisplayDriver), StartupError> {
        use embedded_hal_bus::spi::RefCellDevice;
        use esp_hal::{
            delay::Delay,
            dma_buffers,
            gpio::{Level, Output, OutputConfig},
            spi::master::Config as SpiConfig,
            spi::Mode,
            time::Rate,
        };
        use static_cell::make_static;
        use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
        use embedded_sdmmc::SdCard;

        let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(4092);
        let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer)?;
        let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer)?;

        let spi_config = SpiConfig::default()
            .with_frequency(Rate::from_mhz(10))
            .with_mode(Mode::_0);
        let spi_bus = esp_hal::spi::master::Spi::new(self.spi2, spi_config)?
            .with_sck(self.sck)
            .with_mosi(self.mosi)
            .with_miso(self.miso)
            .with_dma(self.display_dma)
            .with_buffers(dma_rx_buf, dma_tx_buf)
            .into_async();

        let shared_spi_bus = make_static!(core::cell::RefCell::new(spi_bus));

        let card_cs = Output::new(self.card_cs, Level::High, OutputConfig::default());
        let card_spi_device = RefCellDevice::new(shared_spi_bus, card_cs, Delay::new())?;
        let sd_card = SdCard::new(card_spi_device, Delay::new());

        let display_cs = Output::new(self.display_cs, Level::High, OutputConfig::default());
        let display_spi_device = RefCellDevice::new(shared_spi_bus, display_cs, Delay::new())?;

        let bl = Output::new(self.display_bl, Level::High, OutputConfig::default());
        let dc = Output::new(self.display_dc, Level::Low, OutputConfig::default());
        let rst = Output::new(self.display_rst, Level::Low, OutputConfig::default());
        let display = DisplayDriver::new(display_spi_device, bl, rst, dc)?;

        Ok((sd_card, display,))
    }
}