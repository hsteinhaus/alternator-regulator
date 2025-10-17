use super::control::Controller;
use atomic_float::AtomicF32;
use core::cell::RefCell;
use core::fmt;
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use heapless::String;
use num_derive::{FromPrimitive, ToPrimitive};

pub static CONTROLLER: Mutex<CriticalSectionRawMutex, RefCell<Controller>> =
    Mutex::new(RefCell::new(Controller::new()));

pub const RM_LEN: usize = 10;
pub static REGULATOR_MODE: Mutex<CriticalSectionRawMutex, RefCell<String<RM_LEN>>> =
    Mutex::new(RefCell::new(String::new()));

pub const MAX_FIELD_CURRENT: f32 = 3.0;        // A
pub const MAX_FIELD_VOLTAGE: f32 = 20.0;  // V

pub const RPM_MIN: usize = 500;   // rpm (engine)
pub const RPM_MAX: usize = 4500;  // rpm (engine)

#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(FromPrimitive, ToPrimitive, Debug, Default)]
pub enum PpsRunningMode {
    Off = 0,
    Voltage = 1,
    Current = 2,
    #[default]
    Unknown = 3,
}

#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(FromPrimitive, ToPrimitive, Debug, Default)]
pub enum PpsSetMode {
    Off = 0,
    On = 1,
    #[default]
    DontTouch = 2,
}

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
    pub ble_rate: AtomicF32,
    pub target_factor: AtomicF32,
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
    pps_mode: AtomicU8::new(PpsRunningMode::Unknown as u8),
    soc: AtomicF32::new(f32::NAN),
    ble_rate: AtomicF32::new(0.),
    target_factor: AtomicF32::new(0.),
};

#[allow(unused)]
#[derive(Debug)]
pub struct Setpoint {
    pub field_current_limit: AtomicF32,
    pub field_voltage_limit: AtomicF32,
    pub pps_enabled: AtomicU8,
    pub contactor_state: AtomicBool,
}

#[allow(unused)]
pub static SETPOINT: Setpoint = Setpoint {
    field_current_limit: AtomicF32::new(f32::NAN),
    field_voltage_limit: AtomicF32::new(f32::NAN),
    pps_enabled: AtomicU8::new(PpsSetMode::DontTouch as u8),
    contactor_state: AtomicBool::new(false),
};

impl fmt::Display for ProcessData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{};{};{};{};{};{};{};{};{};{}",
            self.rpm.load(Ordering::Relaxed),
            self.target_factor.load(Ordering::Relaxed),
            self.field_current.load(Ordering::Relaxed),
            self.field_voltage.load(Ordering::Relaxed),
            self.bat_current.load(Ordering::Relaxed),
            self.bat_voltage.load(Ordering::Relaxed),
            self.temperature.load(Ordering::Relaxed),
            self.pps_temperature.load(Ordering::Relaxed),
            self.pps_mode.load(Ordering::Relaxed),
            self.ble_rate.load(Ordering::Relaxed),
        )
    }
}

impl fmt::Display for Setpoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{};{};{};{}",
            self.field_current_limit.load(Ordering::Relaxed),
            self.field_voltage_limit.load(Ordering::Relaxed),
            self.pps_enabled.load(Ordering::Relaxed),
            self.contactor_state.load(Ordering::Relaxed),
        )
    }
}

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TemperatureEvent {
    Normal,
    Warning,
    Overheated,
}

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RpmEvent {
    Low,
    HighIdle,
    Normal,
}

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ButtonEvent {
    DecShort(usize),
    DecLong,
    OkShort(usize),
    OkLong,
    IncShort(usize),
    IncLong,
}

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RegulatorEvent {
    Ready,
    Rpm(RpmEvent),
    Button(ButtonEvent),
    Temperature(TemperatureEvent),
}