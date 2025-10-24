//! This module does all the pin- and unit wiring for the board.

use crate::board::io::button::ButtonResources;
use crate::board::io::led::LedResources;
use crate::board::io::pps::PpsResources;
use crate::board::io::radio::RadioResources;
use crate::board::io::rpm::RpmResoures;
use crate::board::io::spi2::Spi2Resources;
use esp_hal::dma::AnySpiDmaChannel;
use esp_hal::gpio::AnyPin;
use esp_hal::i2c::master::AnyI2c;
use esp_hal::peripherals::{Peripherals, CPU_CTRL};
use esp_hal::spi::master::AnySpi;
use esp_hal::{clock::CpuClock, peripherals, rng::Rng, timer::{timg::TimerGroup, AnyTimer}};
use esp_hal::interrupt::software::SoftwareInterruptControl;

#[allow(unused)]
pub struct SystemResources<'a> {
    pub sw_int: SoftwareInterruptControl<'a>,
    pub timer0_1: AnyTimer<'a>,
    pub timer1_0: AnyTimer<'a>,
    pub timer1_1: AnyTimer<'a>,
    pub cpu_ctrl: CPU_CTRL<'a>,
}

pub fn initialize() -> peripherals::Peripherals {
    let var_name = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let config = var_name;
    let peripherals = esp_hal::init(config);

    //esp_alloc::heap_allocator!(size: 4 * 1024);  // 4kB is max for the heap, otherwise "cannot move location counter backwards"
    esp_alloc::heap_allocator!(#[link_section = ".dram2_uninit"] size: 64000); // for WiFi/BLE, even if the rest of the app is statically allocated (min 59000, max 98767)
    peripherals
}

pub fn collect(peripherals: Peripherals) -> (LedResources<'static>, Spi2Resources<'static>, PpsResources<'static>, ButtonResources<'static>, RadioResources<'static>, RpmResoures<'static>, SystemResources<'static>) {
    let led_resources = LedResources {
        core0: AnyPin::from(peripherals.GPIO12),
        core1: AnyPin::from(peripherals.GPIO15),
        user: AnyPin::from(peripherals.GPIO13),
    };

    let spi2_resources = Spi2Resources {
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
    let pps_resources = PpsResources {
        i2c: AnyI2c::from(peripherals.I2C0),
        scl: AnyPin::from(peripherals.GPIO22),
        sda: AnyPin::from(peripherals.GPIO21),
    };
    let rpm_resources = RpmResoures {
        pcnt: peripherals.PCNT,
        pin: AnyPin::from(peripherals.GPIO5),
    };
    let tg0 = TimerGroup::new(peripherals.TIMG0);
    let radio_resources = RadioResources {
        rng: Rng::new(peripherals.RNG),
        wifi: peripherals.WIFI,
        bt: peripherals.BT,
        timer: AnyTimer::from(tg0.timer0),
    };
    let button_resources = ButtonResources {
        button_left: AnyPin::from(peripherals.GPIO39),
        button_center: AnyPin::from(peripherals.GPIO38),
        button_right: AnyPin::from(peripherals.GPIO37),
    };

    let tg1 = TimerGroup::new(peripherals.TIMG1);
    let system_resources = SystemResources {
        sw_int: SoftwareInterruptControl::new(peripherals.SW_INTERRUPT),
        timer0_1: AnyTimer::from(tg0.timer1),
        timer1_0: AnyTimer::from(tg1.timer0),
        timer1_1: AnyTimer::from(tg1.timer1),
        cpu_ctrl: peripherals.CPU_CTRL,
    };

    (
        led_resources,
        spi2_resources,
        pps_resources,
        button_resources,
        radio_resources,
        rpm_resources,
        system_resources,
    )
}
