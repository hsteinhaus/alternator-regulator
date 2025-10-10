use atomic_float::AtomicF32;
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use embassy_time::{with_timeout, Duration, Instant, Ticker};
use num_traits::FromPrimitive;

use crate::board::driver::analog::AdcDriverType;
use crate::board::driver::pps::{Error as PpsError, PpsDriver, RunningMode, SetMode};

pub mod ble_scan;
pub mod rpm;

#[allow(unused)]
#[derive(Debug)]
pub struct ProcessData {
    pub rpm: AtomicF32,
    pub temperature: AtomicF32,
    pub bat_current: AtomicF32,
    pub bat_voltage: AtomicF32,
    pub field_voltage: AtomicF32,
    pub field_current: AtomicF32,
    pub pps_temperature: AtomicF32,
    pub pps_mode: AtomicU8,
    pub soc: AtomicF32,
}

// AtomicF32 is Sync, so this will compile
pub static PROCESS_DATA: ProcessData = ProcessData {
    rpm: AtomicF32::new(f32::NAN),
    temperature: AtomicF32::new(f32::NAN),
    bat_current: AtomicF32::new(f32::NAN),
    bat_voltage: AtomicF32::new(f32::NAN),
    field_voltage: AtomicF32::new(f32::NAN),
    field_current: AtomicF32::new(f32::NAN),
    pps_temperature: AtomicF32::new(f32::NAN),
    pps_mode: AtomicU8::new(RunningMode::Unknown as u8),
    soc: AtomicF32::new(f32::NAN),
};

#[allow(unused)]
#[derive(Debug)]
pub struct Setpoint {
    pub field_current_limit: AtomicF32,
    pub field_voltage_limit: AtomicF32,
    pub pps_enabled: AtomicU8,
    pub contactor_state: AtomicBool,
}

pub static SETPOINT: Setpoint = Setpoint {
    field_current_limit: AtomicF32::new(0.),
    field_voltage_limit: AtomicF32::new(0.),
    pps_enabled: AtomicU8::new(SetMode::DontTouch as u8),
    contactor_state: AtomicBool::new(false),
};

const IO_LOOP_TIME_MS: u64 = 500;

//const HIGH_IDLE_RPM: f32 = 1600. / PULLEY_RATIO;

#[allow(dead_code)]
pub async fn read_adc(adc: &mut AdcDriverType) {
    let r = adc.read_oneshot().await;
    PROCESS_DATA.bat_voltage.store(r as f32, Ordering::SeqCst);
}

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
pub async fn io_task(mut adc: AdcDriverType, mut pps: PpsDriver) -> ! {
    // owned here forever
    let _adc = &mut adc;
    let pps = &mut pps;
    let mut ticker = Ticker::every(Duration::from_millis(IO_LOOP_TIME_MS));
    loop {
        let loop_start = Instant::now();
        trace!("process_data: {:?}", crate::fmt::Debug2Format(&PROCESS_DATA));
        trace!("setpoint: {:?}", crate::fmt::Debug2Format(&SETPOINT));
        with_timeout(Duration::from_millis(IO_LOOP_TIME_MS * 3), async {
            write_pps(pps)
                .await
                .unwrap_or_else(|e| warn!("PPS write error: {:?}", e));
            read_pps(pps).await;
        })
        .await
        .unwrap_or_else(|_| {
            error!("timeout in io i2c loop");
            ticker.reset_at(Instant::now() - Duration::from_millis(IO_LOOP_TIME_MS));
        });
        let loop_time = loop_start.elapsed();
        debug!("io loop time: {:?} ms", loop_time.as_millis());
        ticker.next().await;
    }
}
