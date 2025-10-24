//! Display and sd card are tightly coupled by sharing the same SPI bus.
//! This module therefore handles both in a common task to avoid unnecessary low-level/high-frequency synchronization.

use embassy_executor::Spawner;
use embedded_hal_bus::spi::RefCellDevice;
use embedded_sdmmc::SdCard;
use esp_hal::{
    delay::Delay,
    dma::{AnySpiDmaChannel, DmaBufError, DmaError},
    gpio::{AnyPin, Output},
    spi::master::{AnySpi, SpiDmaBus},
    Async,
};
use thiserror_no_std::Error;

use crate::app::logger::logger_loop;
use crate::board::driver::display::{DisplayDriver, DisplayError};
use crate::fmt::Debug2Format;
use crate::ui::ui_task;

type SpiBusType = SpiDmaBus<'static, Async>;
pub type SpiDeviceType<'a> = RefCellDevice<'a, SpiBusType, Output<'static>, Delay>;
pub type SdCardType = SdCard<SpiDeviceType<'static>, Delay>;


#[derive(Debug, Error)]
pub enum Spi2Error {
    #[error("Display initialization failed: {0:?}")]
    DisplayInitFailed(#[from] DisplayError),

    #[error("DMA initialization failed: {0:?}")]
    DmaInitFailed(#[from] DmaBufError),

    #[error("DMA error: {0:?}")]
    DmaError(#[from] DmaError),

    #[error("SPI Master config error: {0:?}")]
    SpiConfigError(#[from] esp_hal::spi::master::ConfigError),

    #[error("Never happened")]
    NeverHappened(#[from] core::convert::Infallible),
}

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
    pub fn into_devices(self) -> Result<(SdCardType, DisplayDriver), Spi2Error> {
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


#[embassy_executor::task]
pub async fn spi2_task(spawner: Spawner, spi2_resources: Spi2Resources<'static>) -> () {
    let (sd_card, display) = match spi2_resources.into_devices() {
        Ok(drivers) => drivers,
        Err(err) => {
            error!("Failed to initialize SPI2 devices: {:?}", Debug2Format(&err));
            return;
        }
    };
    spawner.must_spawn(ui_task(display));
    logger_loop(sd_card).await;
}