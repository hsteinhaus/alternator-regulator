use core::sync::atomic::{Ordering};
use atomic_float::AtomicF32;
use defmt::{debug, info};
use embassy_time::{Duration, Timer};

use crate::board::driver::analog::AdcDriverType;
use crate::board::driver::pcnt::PcntDriver;
use crate::board::driver::pps::PpsDriver;

pub mod ble_scan;

#[allow(unused)]
pub struct ProcessData {
    pub rpm: AtomicF32,
    pub temperature: AtomicF32,
    pub voltage: AtomicF32,
    pub current: AtomicF32,
    pub field_voltage: AtomicF32,
    pub field_current: AtomicF32,
    pub soc: AtomicF32,
}

// AtomicF32 is Sync, so this will compile
pub static PROCESS_DATA: ProcessData = ProcessData {
    rpm: AtomicF32::new(f32::NAN),
    temperature: AtomicF32::new(f32::NAN),
    voltage: AtomicF32::new(f32::NAN),
    current: AtomicF32::new(f32::NAN),
    field_voltage: AtomicF32::new(f32::NAN),
    field_current: AtomicF32::new(f32::NAN),
    soc: AtomicF32::new(f32::NAN),
};

#[embassy_executor::task]
pub async fn adc_task(mut adc: AdcDriverType) -> ! {
    loop {
        let r = adc.read_oneshot().await;
        info!("adc: {}", r);
        Timer::after(Duration::from_millis(4900)).await;
    }
}

#[embassy_executor::task]
pub async fn rpm_task(mut pcnt_driver: PcntDriver) -> ! {
    const POLE_PAIRS: f32 = 6.;
    const LOOP_DELAY_MS: u64 = 100;
    const PULLEY_RATIO: f32 = 53.7 / 128.2;

    loop {
        let pulse_count = pcnt_driver.get_and_reset();
        let rpm = pulse_count as f32
            * 60.                            // Hz -> rpm
            * (1./POLE_PAIRS/2.)            // 6 pole pairs, 2 imp per rev
            * (1000./LOOP_DELAY_MS as f32)   // interval
            * PULLEY_RATIO; // belt ratio
        PROCESS_DATA.rpm.store(rpm, Ordering::Relaxed);
        Timer::after(Duration::from_millis(LOOP_DELAY_MS)).await;
    }
}


//#[esp_hal_embassy::main]
#[embassy_executor::task]
pub async fn pps_task(mut pps: PpsDriver) -> () {
    loop {
        let v_out = pps.get_voltage().unwrap_or(f32::NAN);
        let v_in = pps.get_input_voltage().unwrap_or(f32::NAN);
        let i_field = pps.get_current().unwrap_or(f32::NAN);

        debug!(
            "PPS state: mode: {:?}, T: {:?}, Vi: {:?}, Vo: {:?}, Io: {:?}",
            pps.get_running_mode(),
            pps.get_temperature(),
            v_in,
            v_out,
            i_field,
        );
        PROCESS_DATA.field_voltage.store(v_out, Ordering::Relaxed);
        Timer::after(Duration::from_millis(100)).await;
    }
}
