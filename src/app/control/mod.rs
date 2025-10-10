use core::cmp::min;
use core::sync::atomic::Ordering;
use libm::{floorf, fmaxf};

use crate::app::shared::PROCESS_DATA;

const RPM_MIN: usize = 1200;
const RPM_MAX: usize = 4500;
const RPM_STEP: usize = 100;
const RPM_ARRAY_SIZE: usize = (RPM_MAX - RPM_MIN) / RPM_STEP;
const RPM_FACTOR: [f32; RPM_ARRAY_SIZE] = Controller::calc_rpm_factor_array();

#[derive(Debug)]
pub struct Controller {
    _last_current: f32,
}

impl Controller {
    const fn calc_rpm_factor_array<const SIZE: usize>() -> [f32; SIZE] {
        let res = [1.0_f32; SIZE];
        res
    }

    fn lookup_rpm_factor(rpm: f32) -> f32 {
        // Normalize RPM to array index (0.0 to RPM_ARRAY_SIZE-1)
        let rpm = fmaxf((rpm - (RPM_MIN as f32)) / (RPM_STEP as f32), 0.);
        let rpm_floor = min(floorf(rpm) as usize, RPM_ARRAY_SIZE - 2);

        let f0 = RPM_FACTOR[rpm_floor];
        let f1 = RPM_FACTOR[rpm_floor + 1];

        // Linear interpolation between f0 and f1
        let f = f0 + (f1 - f0) * (rpm - rpm_floor as f32);
        f
    }

    pub const fn new() -> Self {
        Self { _last_current: 0. }
    }

    pub fn start_charging(&mut self) {
        info!("starting charging");
    }

    pub fn stop_charging(&mut self) {
        info!("stopping charging");
    }

    pub fn adjust_current(&mut self, increment: f32) {
        debug!(
            "{}",
            Controller::lookup_rpm_factor(PROCESS_DATA.rpm.load(Ordering::SeqCst) as f32)
        );
        info!("increasing current by {} A", (increment as f32) * 0.1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_rpm_factor_min() {
        error!("TESTING!");
        assert_eq!(Controller::lookup_rpm_factor(RPM_MIN as f32), RPM_FACTOR[0]);
    }

    #[test]
    fn test_lookup_rpm_factor_max() {
        assert_eq!(
            Controller::lookup_rpm_factor(RPM_MAX as f32),
            RPM_FACTOR[RPM_ARRAY_SIZE - 1]
        );
    }

    #[test]
    fn test_lookup_rpm_factor_interpolation() {
        let rpm = RPM_MIN as f32 + RPM_STEP as f32 / 2.0;
        let expected = (RPM_FACTOR[0] + RPM_FACTOR[1]) / 2.0;
        assert!((Controller::lookup_rpm_factor(rpm) - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn test_lookup_rpm_factor_below_min() {
        assert_eq!(Controller::lookup_rpm_factor((RPM_MIN as f32) - 100.0), RPM_FACTOR[0]);
    }

    #[test]
    fn test_lookup_rpm_factor_above_max() {
        assert_eq!(
            Controller::lookup_rpm_factor((RPM_MAX as f32) + 100.0),
            RPM_FACTOR[RPM_ARRAY_SIZE - 1]
        );
    }
}
