use core::sync::atomic::Ordering;
use embassy_time::{with_timeout, Duration, Instant, Ticker};
use num_traits::FromPrimitive;
use crate::app::shared::{PROCESS_DATA, SETPOINT};
use crate::board::driver::analog::AdcDriverType;
use crate::board::driver::pps::{Error as PpsError, PpsDriver, SetMode};

const PPS_LOOP_TIME_MS: u64 = 500;

pub async fn read_pps(pps: &mut PpsDriver) {
    pps.get_voltage()
        .and_then(|v| Ok(PROCESS_DATA.field_voltage.store(v, Ordering::SeqCst)))
        .ok();
    pps.get_current()
        .and_then(|i| Ok(PROCESS_DATA.field_current.store(i, Ordering::SeqCst)))
        .ok();
    pps.get_temperature()
        .and_then(|t| Ok(PROCESS_DATA.pps_temperature.store(t, Ordering::SeqCst)))
        .ok();
    pps.get_running_mode()
        .and_then(|m| Ok(PROCESS_DATA.pps_mode.store(m as u8, Ordering::SeqCst)))
        .ok();
}

pub async fn write_pps(pps: &mut PpsDriver) -> Result<(), PpsError> {
    let cl = SETPOINT.field_current_limit.swap(f32::NAN, Ordering::SeqCst);
    let vl = SETPOINT.field_voltage_limit.swap(f32::NAN, Ordering::SeqCst);
    let enabled = SetMode::from_u8(SETPOINT.pps_enabled.swap(SetMode::DontTouch as u8, Ordering::SeqCst))
        .ok_or(PpsError::Unsupported)?;
    debug!("write_pps: cl: {:?} vl: {:?} enabled: {:?}", cl, vl, enabled);

    if !cl.is_nan() {
        pps.set_current(cl)?;
    }
    if !vl.is_nan() {
        pps.set_voltage(vl)?;
    }
    match enabled {
        SetMode::Off => {
            pps.enable(false)?;
        }
        SetMode::On => {
            pps.enable(true)?;
        }
        SetMode::DontTouch => (),
    };
    Ok(())
}

#[embassy_executor::task]
pub async fn pps_task(mut adc: AdcDriverType, mut pps: PpsDriver) -> ! {
    // owned here forever
    let _adc = &mut adc;
    let pps = &mut pps;
    let mut ticker = Ticker::every(Duration::from_millis(PPS_LOOP_TIME_MS));
    loop {
        let loop_start = Instant::now();
        trace!("process_data: {:?}", crate::fmt::Debug2Format(&PROCESS_DATA));
        trace!("setpoint: {:?}", crate::fmt::Debug2Format(&SETPOINT));
        with_timeout(Duration::from_millis(PPS_LOOP_TIME_MS * 3), async {
            write_pps(pps)
                .await
                .unwrap_or_else(|e| warn!("PPS write error: {:?}", e));
            read_pps(pps).await;
        })
        .await
        .unwrap_or_else(|_| {
            error!("timeout in io i2c loop");
            ticker.reset_at(Instant::now() - Duration::from_millis(PPS_LOOP_TIME_MS));
        });
        let loop_time = loop_start.elapsed();
        debug!("io loop time: {:?} ms", loop_time.as_millis());
        ticker.next().await;
    }
}