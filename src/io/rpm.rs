use core::sync::atomic::Ordering;
use embassy_time::{Duration, Ticker};

use crate::board::driver::pcnt::PcntDriver;
use crate::io::{ProcessData, PROCESS_DATA};
use crate::state::{RegulatorEvent, RpmEvent, SenderType};
use crate::util::zc::detect_zero_crossing_with_hysteresis;

const RPM_LOOP_TIME_MS: u64 = 100;
const POLE_PAIRS: f32 = 6.;
const PULLEY_RATIO: f32 = 53.7 / 128.2;
const LOW_RPM_THRESHOLD: f32 = 1200.;

pub async fn read_rpm(pcnt_driver: &mut PcntDriver) -> f32 {
    let pulse_count = pcnt_driver.get_and_reset();
    let rpm = pulse_count as f32
        * 60.                              // Hz -> rpm
        * (1./POLE_PAIRS/2.)               // 6 pole pairs, 2 imp per rev
        * (1000./RPM_LOOP_TIME_MS as f32)  // intervals per second
        * PULLEY_RATIO; // belt ratio
    PROCESS_DATA.rpm.store(rpm, Ordering::SeqCst);
    rpm
}

#[embassy_executor::task]
pub async fn rpm_task(sender: SenderType, mut pcnt_driver: PcntDriver) -> ! {
    // take ref to avoid a move in the loop iteration (value is owned in this fn forever)
    let pcnt_driver = &mut pcnt_driver;

    let mut above = false;
    let mut crossed;

    let mut ticker = Ticker::every(Duration::from_millis(RPM_LOOP_TIME_MS));
    loop {
        let rpm = read_rpm(pcnt_driver).await;
        (above, crossed) = detect_zero_crossing_with_hysteresis(rpm, LOW_RPM_THRESHOLD, 0.05, above);
        if crossed {
            let event = if above { RpmEvent::Normal } else { RpmEvent::Low };
            sender.send(RegulatorEvent::Rpm(event)).await;
            debug!("sending rpm event: {:?}", event);
        }
        ticker.next().await;
    }
}

impl ProcessData {
    pub fn rpm_is_normal(&self) -> bool {
        self.rpm.load(Ordering::SeqCst) > LOW_RPM_THRESHOLD
    }
}
