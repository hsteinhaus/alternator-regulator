use crate::app::shared::{PpsSetMode, BASE_CURRENT, CONTROLLER, MAX_FIELD_VOLTAGE, PROCESS_DATA, RPM_MAX, RPM_MIN, SETPOINT};
use core::cmp::min;
use core::sync::atomic::Ordering;
use embassy_time::{Duration, Ticker};
use libm::{floorf, fmaxf};

#[derive(Debug)]
pub struct Controller {
    _last_current: f32,
    target: f32,
    derating: f32,
    power_on: bool,
}

#[allow(unused)]
impl Controller {
    const RPM_STEP: usize = 100;
    const RPM_ARRAY_SIZE: usize = RPM_MAX / Controller::RPM_STEP;
    const RPM_FACTOR: [f32; Controller::RPM_ARRAY_SIZE] = Controller::const_rpm_factor();
    const LOOP_INTERVAL_MS: u64 = 100;

    pub const fn new() -> Self {
        Self {
            _last_current: 0.,
            derating: 1.,
            power_on: false,
            target: 0.,
        }
    }

    pub fn adjust_target(&mut self, target: f32) {
        assert!(target >= 0. && target <= 1.);
        info!("adjusting target to {}", target);
        self.target = target;
    }

    pub fn adjust_target_inc(&mut self, target_inc: f32) {
        assert!(target_inc >= 0. && target_inc <= 1.);
        let mut target = self.target + target_inc;
        if target > 1. {
            target = 1.
        };
        if target < 0. {
            target = 0.
        };
        self.target = target;
        info!("adjusting target to {}", target);
        self.target = target;
    }

    pub fn set_derating(&mut self, derating: f32) {
        assert!(derating >= 0. && derating <= 1.);
        info!("setting derating to {}", derating);
        self.derating = derating;
    }

    pub fn update(&self) {
        if self.power_on {
            let rpm = PROCESS_DATA.rpm.load(Ordering::Relaxed) as f32;
            let f = BASE_CURRENT /* * Self::lookup_rpm_factor(rpm)*/;
            let target = f * self.target * self.derating;
            SETPOINT.field_current_limit.store(target, Ordering::SeqCst);
        }
    }

    pub fn start_charging(&mut self) {
        info!("starting charging");
        SETPOINT.pps_enabled.store(PpsSetMode::On as u8, Ordering::SeqCst);
        SETPOINT.field_voltage_limit.store(MAX_FIELD_VOLTAGE, Ordering::SeqCst);
        self.power_on = true;
        self.target = 0.1;
    }

    pub fn stop_charging(&mut self) {
        info!("stopping charging");
        SETPOINT.pps_enabled.store(PpsSetMode::Off as u8, Ordering::SeqCst);;
        self.power_on = false;
    }

    fn lookup_rpm_factor(rpm: f32) -> f32 {
        // Normalize RPM to array index (0.0 to RPM_ARRAY_SIZE-1)
        let index_rpm = rpm / Controller::RPM_STEP as f32; //

        let index_fl = fmaxf(floorf(index_rpm), 0.) as usize; // > 0
        let index_fl_bounded = min(index_fl, Controller::RPM_ARRAY_SIZE - 2); // < RPM_ARRAY_SIZE - 2

        let f0 = Controller::RPM_FACTOR[index_fl_bounded];
        let f1 = Controller::RPM_FACTOR[index_fl_bounded + 1];

        // Linear interpolation between f0 and f1
        let f = f0 + (f1 - f0) * (index_rpm - index_fl as f32);
        debug!("rpm factor lookup: {}, f0: {}, f1: {} -> f: {}", rpm, f0, f1, f);
        f
    }

    const fn const_rpm_factor<const SIZE: usize>() -> [f32; SIZE] {

        // first guess: double RPM, double current
        const fn calc_rpm_factor(array_index: usize) -> f32 {
            let rpm_index = RPM_MIN / Controller::RPM_STEP;
            rpm_index as f32 / array_index as f32
        }

        let mut tmp = [0.; SIZE];
        let mut i = 0;

        // no for loop in const expressions :-|
        while i < SIZE {
            tmp[i] = calc_rpm_factor(i);
            i += 1;
        }
        tmp
    }
}

#[embassy_executor::task]
pub async fn controller_task() -> ! {
    let mut ticker = Ticker::every(Duration::from_millis(Controller::LOOP_INTERVAL_MS));
    loop {
        CONTROLLER.lock(move |c| {
            c.borrow().update();
        });
        ticker.next().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_rpm_factor_min() {
        error!("TESTING!");
        assert_eq!(Controller::lookup_rpm_factor(RPM_MIN as f32), Controller::RPM_FACTOR[0]);
    }

    #[test]
    fn test_lookup_rpm_factor_max() {
        assert_eq!(
            Controller::lookup_rpm_factor(RPM_MAX as f32),
            Controller::RPM_FACTOR[Controller::RPM_ARRAY_SIZE - 1]
        );
    }

    #[test]
    fn test_lookup_rpm_factor_interpolation() {
        let rpm = RPM_MIN as f32 + Controller::RPM_STEP as f32 / 2.0;
        let expected = (Controller::RPM_FACTOR[0] + Controller::RPM_FACTOR[1]) / 2.0;
        assert!((Controller::lookup_rpm_factor(rpm) - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn test_lookup_rpm_factor_below_min() {
        assert_eq!(Controller::lookup_rpm_factor((RPM_MIN as f32) - 100.0), Controller::RPM_FACTOR[0]);
    }

    #[test]
    fn test_lookup_rpm_factor_above_max() {
        assert_eq!(
            Controller::lookup_rpm_factor((RPM_MAX as f32) + 100.0),
            Controller::RPM_FACTOR[Controller::RPM_ARRAY_SIZE - 1]
        );
    }
}
