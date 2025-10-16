use core::sync::atomic::Ordering;

use crate::app::shared::PROCESS_DATA;
use crate::board::driver::analog::AdcDriverType;

pub mod ble_scan;
pub mod button;
pub mod pps;
pub mod rpm;

#[allow(dead_code)]
pub async fn read_adc(adc: &mut AdcDriverType) {
    let r = adc.read_oneshot().await;
    PROCESS_DATA.bat_voltage.store(r as f32, Ordering::Relaxed);
}
