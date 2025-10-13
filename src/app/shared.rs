use atomic_float::AtomicF32;
use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, AtomicU8};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;

use super::control::Controller;
use crate::board::driver::pps::{RunningMode, SetMode};

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

pub static CONTROLLER: Mutex<CriticalSectionRawMutex, RefCell<Controller>> =
    Mutex::new(RefCell::new(Controller::new()));
pub const RPM_MIN: usize = 1200;
pub const RPM_MAX: usize = 4500;
pub const MAX_FIELD_VOLTAGE: f32 = 20.0;