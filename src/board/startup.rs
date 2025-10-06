use async_button::{Button, ButtonConfig};
use crate::board::driver::{
    analog::{AdcDriver, AdcDriverType},
    display::DisplayDriver,
    pcnt::PcntDriver,
    pps::PpsDriver,
    wifi_ble::WifiDriver,
};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::system::CpuControl;
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
use esp_hal::rng::Rng;

#[allow(dead_code)]
pub struct Resources<'a> {
    pub led0: Output<'a>,
    pub led1: Output<'a>,
    pub button_left: Button<Input<'a>>,
    pub button_center: Button<Input<'a>>,
    pub button_right: Button<Input<'a>>,
    pub rng: Rng,
    pub display: DisplayDriver,
    pub wifi_ble: WifiDriver,
    pub pps: PpsDriver,
    pub pcnt: PcntDriver,
    pub adc: AdcDriverType,
    pub cpu_control: CpuControl<'a>,
}

impl <'a> Resources<'a> {
    pub fn initialize() -> Self {
        let var_name = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
        let config = var_name;
        let peripherals = esp_hal::init(config);

        //esp_alloc::heap_allocator!(size: 4 * 1024);  // 4kB is max for the heap, otherwise "cannot move location counter backwards"
        esp_alloc::heap_allocator!(#[link_section = ".dram2_uninit"] size: 98767); // for WiFi/BLE, even if the rest of the app is statically allocated

        let timer0 = TimerGroup::new(peripherals.TIMG1);
        esp_hal_embassy::init(timer0.timer0);


        /////////////////////////// GPIO init ////////////////////////////
        let led0 = Output::new(peripherals.GPIO12, Level::Low, OutputConfig::default());
        let led1 = Output::new(peripherals.GPIO15, Level::Low, OutputConfig::default());
        let rng = Rng::new(peripherals.RNG);

        let button_left = Input::new(peripherals.GPIO39, InputConfig::default().with_pull(Pull::Up));
        let button_left = Button::new(button_left, ButtonConfig::default());

        let button_center = Input::new(peripherals.GPIO38, InputConfig::default().with_pull(Pull::Up));
        let button_center = Button::new(button_center, ButtonConfig::default());

        let button_right = Input::new(peripherals.GPIO37, InputConfig::default().with_pull(Pull::Up));
        let button_right = Button::new(button_right, ButtonConfig::default());

        ////////////////////////// Display init ////////////////////////////
        let sclk = peripherals.GPIO18;
        let mosi = peripherals.GPIO23;
        let cs = peripherals.GPIO14;
        let bl = peripherals.GPIO32;

        #[allow(clippy::manual_div_ceil)]
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
                .with_timeout(BusTimeout::BusCycles(20)),
        )
        .unwrap()
        .with_sda(peripherals.GPIO21)
        .with_scl(peripherals.GPIO22)
        .into_async();
        let pps = PpsDriver::new(i2c, 0x35).expect("PPS module init failed");

        ////////////////////////// Pulse counter init ////////////////////////////
        let rpm_pin = Input::new(
            peripherals.GPIO5,
            InputConfig::default().with_pull(esp_hal::gpio::Pull::Down),
        );
        let pcnt = PcntDriver::initialize(peripherals.PCNT, rpm_pin).expect("PCNT module init failed");

        ////////////////////////// ADC init ////////////////////////////
        let adc = AdcDriver::initialize(peripherals.ADC2, peripherals.GPIO26);

        ////////////////////////// WiFi & BLE init ////////////////////////////
        let wifi_driver = crate::board::driver::wifi_ble::WifiDriver::new(
            peripherals.WIFI,
            peripherals.BT,
            AnyTimer::from(TimerGroup::new(peripherals.TIMG0).timer0),
            rng,
        );

        Self {
            led0,
            led1,
            button_left,
            button_center,
            button_right,
            rng,
            display: d,
            wifi_ble: wifi_driver,
            pps,
            pcnt,
            adc,
            cpu_control: CpuControl::new(peripherals.CPU_CTRL),
        }
    }
}
