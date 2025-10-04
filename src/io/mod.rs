use core::sync::atomic::{AtomicBool, Ordering};
use atomic_float::AtomicF32;
use defmt::{debug, info, warn, Debug2Format};
use embassy_time::{Duration, Timer};
use crate::board::driver::analog::AdcDriverType;
use crate::board::driver::pcnt::PcntDriver;
use crate::board::driver::pps::{PpsDriver, Error as PpsError};

pub mod ble_scan;

#[allow(unused)]
#[derive(Debug)]
pub struct ProcessData {
    pub rpm: AtomicF32,
    pub temperature: AtomicF32,
    pub voltage: AtomicF32,
    pub current: AtomicF32,
    pub bat_voltage: AtomicF32,
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
    bat_voltage: AtomicF32::new(f32::NAN),
    field_voltage: AtomicF32::new(f32::NAN),
    field_current: AtomicF32::new(f32::NAN),
    soc: AtomicF32::new(f32::NAN),
};

#[allow(unused)]
#[derive(Debug)]
pub struct Setpoint {
    pub field_current_limit: AtomicF32,
    pub field_voltage_limit: AtomicF32,
    pub pps_enabled: AtomicBool,
    pub contactor_state: AtomicBool,
}

pub static SETPOINT: Setpoint = Setpoint {
    field_current_limit: AtomicF32::new(0.),
    field_voltage_limit: AtomicF32::new(0.),
    pps_enabled: AtomicBool::new(false),
    contactor_state: AtomicBool::new(false),
};



#[embassy_executor::task]
pub async fn io_task(mut adc: AdcDriverType, mut pcnt_driver: PcntDriver, mut pps: PpsDriver) -> ! {
    // owned here forever
    let _adc = &mut adc;
    let pcnt_driver = &mut pcnt_driver;
    let pps = &mut pps;
    loop {
        info!("setpoint: {:?}", Debug2Format(&SETPOINT));
        write_pps(pps).await.unwrap_or_else(|e| warn!("PPS write error: {:?}", e));

        info!("process_data: {:?}", Debug2Format(&PROCESS_DATA));
        //read_adc(adc).await;
        read_rpm(pcnt_driver).await;
        read_pps(pps).await;
        Timer::after(Duration::from_millis(100)).await;
    }
}

#[allow(dead_code)]
pub async fn read_adc(adc: &mut AdcDriverType) {
        let r = adc.read_oneshot().await;
        PROCESS_DATA.bat_voltage.store(r as f32, Ordering::SeqCst);
}

pub async fn read_rpm(pcnt_driver: &mut PcntDriver) {
    const POLE_PAIRS: f32 = 6.;
    const LOOP_DELAY_MS: u64 = 100;
    const PULLEY_RATIO: f32 = 53.7 / 128.2;

    let pulse_count = pcnt_driver.get_and_reset();
    let rpm = pulse_count as f32
        * 60.                            // Hz -> rpm
        * (1./POLE_PAIRS/2.)            // 6 pole pairs, 2 imp per rev
        * (1000./LOOP_DELAY_MS as f32)   // interval
        * PULLEY_RATIO; // belt ratio
    PROCESS_DATA.rpm.store(rpm, Ordering::SeqCst);
}

pub async fn read_pps(pps: &mut PpsDriver) {
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
    PROCESS_DATA.field_voltage.store(v_out, Ordering::SeqCst);
}

pub async fn write_pps(pps: &mut PpsDriver) -> Result<(), PpsError>{
    let cl = SETPOINT.field_current_limit.load(Ordering::SeqCst);
    let vl = SETPOINT.field_voltage_limit.load(Ordering::SeqCst);
    let enabled = SETPOINT.pps_enabled.load(Ordering::SeqCst);

    if !cl.is_nan() && !vl.is_nan() && enabled {
        pps.set_current(cl)?.set_voltage(vl)?.enable(true)?;
    }
    Ok(())
}