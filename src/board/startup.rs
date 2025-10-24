use crate::board::driver::display::DisplayError;
use crate::board::driver::wifi_ble::WifiError;
use crate::board::driver::{
    display::DisplayDriver,
    pcnt::PcntDriver,
    pps::PpsDriver,
    wifi_ble::WifiDriver,
};
use crate::util::led_debug::LedDebug;
use async_button::{Button, ButtonConfig};
use embedded_hal_bus::spi::RefCellDevice;
use embedded_sdmmc::SdCard;
use esp_hal::dma::{AnySpiDmaChannel, DmaBufError, DmaError};
use esp_hal::gpio::AnyPin;
use esp_hal::i2c::master::AnyI2c;
use esp_hal::spi::master::AnySpi;
use esp_hal::{clock::CpuClock, delay::Delay, dma::{DmaRxBuf, DmaTxBuf}, dma_buffers, gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull}, i2c::master::{BusTimeout, Config as I2cConfig, I2c}, peripherals, rng::Rng, spi::master::SpiDmaBus, system::CpuControl, time::Rate, timer::{timg::TimerGroup, AnyTimer}, Async};
use static_cell::make_static;
use thiserror_no_std::Error;

type SpiBusType = SpiDmaBus<'static, Async>;
pub type SpiDeviceType<'a> = RefCellDevice<'a, SpiBusType, Output<'static>, Delay>;
pub type SdCardType = SdCard<SpiDeviceType<'static>, Delay>;

#[derive(Debug, Error)]
pub enum StartupError {
    #[error("Display initialization failed: {0:?}")]
    DisplayInitFailed(#[from] DisplayError),

    #[error("DMA initialization failed: {0:?}")]
    DmaInitFailed(#[from] DmaBufError),

    #[error("DMA error: {0:?}")]
    DmaError(#[from] DmaError),

    #[error("I2C Master error: {0:?}")]
    I2cError(#[from] esp_hal::i2c::master::ConfigError),

    #[error("PPS error: {0:?}")]
    PpsError(#[from] crate::board::driver::pps::PpsError),

    #[error("SPI Master config error: {0:?}")]
    SpiConfigError(#[from] esp_hal::spi::master::ConfigError),

    #[error("PCNT error: {0:?}")]
    PcntError(#[from] crate::board::driver::pcnt::Error),

    #[error("Wifi error: {0:?}")]
    WifiError(#[from] WifiError),

    #[error("Never happened")]
    NeverHappened(#[from] core::convert::Infallible),
}

#[allow(unused)]
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


struct ButtonResources<'a> {
    button_left: AnyPin<'a>,
    button_center: AnyPin<'a>,
    button_right: AnyPin<'a>,
}

impl<'a> ButtonResources<'a> {
    pub fn into_buttons(self) -> (Button<Input<'a>>, Button<Input<'a>>, Button<Input<'a>>) {
        let button_left = Input::new(self.button_left, InputConfig::default().with_pull(Pull::Up));
        let button_left = Button::new(button_left, ButtonConfig::default());

        let button_center = Input::new(self.button_center, InputConfig::default().with_pull(Pull::Up));
        let button_center = Button::new(button_center, ButtonConfig::default());

        let button_right = Input::new(self.button_right, InputConfig::default().with_pull(Pull::Up));
        let button_right = Button::new(button_right, ButtonConfig::default());
        (button_left, button_center, button_right)
    }
}


struct PpsResources<'a> {
    i2c: AnyI2c<'a>,
    scl: AnyPin<'a>,
    sda: AnyPin<'a>,
}

impl PpsResources<'static> {
    pub(crate) fn into_pps(self) -> Result<PpsDriver, StartupError> {
        let i2c = I2c::new(
            self.i2c,
            I2cConfig::default()
                .with_frequency(Rate::from_khz(400))
                .with_timeout(BusTimeout::BusCycles(20)),
        )?
            .with_sda(self.sda)
            .with_scl(self.scl)
            .into_async();
        let pps = PpsDriver::new(i2c, 0x35)?;
        Ok(pps)
    }
}

struct RadioResources<'a> {
    rng: Rng,
    wifi: peripherals::WIFI<'a>,
    bt: peripherals::BT<'a>,
    timer: AnyTimer<'a>,
}

impl RadioResources<'static> {
    pub fn into_driver(self) -> Result<WifiDriver, StartupError> {
        Ok(WifiDriver::new(self.wifi, self.bt, self.timer, self.rng)?)
    }
}

#[allow(unused)]
pub struct Resources {
    pub led0: Output<'static>,
    pub led1: Output<'static>,
    pub button_left: Button<Input<'static>>,
    pub button_center: Button<Input<'static>>,
    pub button_right: Button<Input<'static>>,
    pub display: DisplayDriver,
    pub wifi_ble: WifiDriver,
    pub pps: PpsDriver,
    pub pcnt: PcntDriver,
    pub cpu_control: CpuControl<'static>,
    pub sd_card: SdCardType,
}

impl Resources {

    pub fn initialize() -> Result<Self, StartupError> {
        let var_name = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
        let config = var_name;
        let peripherals = esp_hal::init(config);

        //esp_alloc::heap_allocator!(size: 4 * 1024);  // 4kB is max for the heap, otherwise "cannot move location counter backwards"
        esp_alloc::heap_allocator!(#[link_section = ".dram2_uninit"] size: 64000); // for WiFi/BLE, even if the rest of the app is statically allocated (min 59000, max 98767)
        let timer0 = TimerGroup::new(peripherals.TIMG1);
        esp_hal_embassy::init(timer0.timer0);


        let led0 = Output::new(peripherals.GPIO12, Level::Low, OutputConfig::default());
        let led1 = Output::new(peripherals.GPIO15, Level::Low, OutputConfig::default());
        let led2 = Output::new(peripherals.GPIO13, Level::Low, OutputConfig::default());
        LedDebug::create(led2);


        let spi2_peripherals = Spi2Resources {
            spi2: AnySpi::from(peripherals.SPI2),
            display_dma: AnySpiDmaChannel::from(peripherals.DMA_SPI2),
            sck: AnyPin::from(peripherals.GPIO18),
            mosi: AnyPin::from(peripherals.GPIO23),
            miso: AnyPin::from(peripherals.GPIO19),
            card_cs: AnyPin::from(peripherals.GPIO4),
            display_cs: AnyPin::from(peripherals.GPIO14),
            display_bl: AnyPin::from(peripherals.GPIO32),
            display_dc: AnyPin::from(peripherals.GPIO27),
            display_rst: AnyPin::from(peripherals.GPIO33),
        };
        let (sd_card, display) = spi2_peripherals.into_devices()?;

        let pps_resources = PpsResources {
            i2c: AnyI2c::from(peripherals.I2C0),
            scl: AnyPin::from(peripherals.GPIO22),
            sda: AnyPin::from(peripherals.GPIO21),
        };
        let pps = pps_resources.into_pps()?;

        let button_resources = ButtonResources {
            button_left: AnyPin::from(peripherals.GPIO39),
            button_center: AnyPin::from(peripherals.GPIO38),
            button_right: AnyPin::from(peripherals.GPIO37),
        };
        let (button_left, button_center, button_right) = ButtonResources::into_buttons(button_resources);


        let rpm_pin = Input::new(
            peripherals.GPIO5,
            InputConfig::default().with_pull(esp_hal::gpio::Pull::Down),
        );
        let pcnt = PcntDriver::new(peripherals.PCNT, rpm_pin)?;

        ////////////////////////// WiFi & BLE init ////////////////////////////
        let radio_resources = RadioResources {
            rng: Rng::new(peripherals.RNG),
            wifi: peripherals.WIFI,
            bt: peripherals.BT,
            timer: AnyTimer::from(TimerGroup::new(peripherals.TIMG0).timer0),
        };
        let wifi_driver = radio_resources.into_driver()?;

        Ok(Self {
            led0,
            led1,
            button_left,
            button_center,
            button_right,
            display,
            wifi_ble: wifi_driver,
            pps,
            pcnt,
            cpu_control: CpuControl::new(peripherals.CPU_CTRL),
            sd_card,
        })
    }
}
