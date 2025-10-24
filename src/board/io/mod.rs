use core::sync::atomic::Ordering;

use crate::app::shared::PROCESS_DATA;
use crate::board::driver::analog::AdcDriverType;

pub mod button;
pub mod led;
pub mod pps;
pub mod radio;
pub mod rpm;
pub mod spi2;

#[allow(dead_code)]
pub async fn read_adc(adc: &mut AdcDriverType) {
    let r = adc.read_oneshot().await;
    PROCESS_DATA.bat_voltage.store(r as f32, Ordering::Relaxed);
}
