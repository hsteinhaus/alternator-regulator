use atomic_float::AtomicF32;
use core::cell::RefCell;
use core::fmt;
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use heapless::{format, String};
use num_derive::{FromPrimitive, ToPrimitive};
use embassy_sync::channel::{Channel, Receiver, Sender};
use static_cell::StaticCell;
use super::control::Controller;
use crate::app::logger::{LoggerMeta, LINE_LEN};

pub static CONTROLLER: Mutex<CriticalSectionRawMutex, RefCell<Controller>> =
    Mutex::new(RefCell::new(Controller::new()));

pub const RM_LEN: usize = 10;
pub static REGULATOR_MODE: Mutex<CriticalSectionRawMutex, RefCell<String<RM_LEN>>> =
    Mutex::new(RefCell::new(String::new()));

pub const MAX_FIELD_CURRENT: f32 = 3.0; // A
pub const MAX_FIELD_VOLTAGE: f32 = 20.0; // V

pub const RPM_MIN: usize = 500; // rpm (engine)
pub const RPM_MAX: usize = 4500; // rpm (engine)

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

/// All external data that are observed by the regulator
///
/// This struct is filled by various sources, mostly drivers. The data are logged to SD card if
/// present.
#[allow(unused)]
#[derive(Debug)]
pub struct ProcessData {
    pub rpm: AtomicF32,
    pub temperature: AtomicF32,
    pub bat_current: AtomicF32,
    pub bat_soc: AtomicF32,
    pub bat_voltage: AtomicF32,
    pub input_voltage: AtomicF32,
    pub field_voltage: AtomicF32,
    pub field_current: AtomicF32,
    pub pps_temperature: AtomicF32,
    pub pps_mode: AtomicU8,
    pub ble_rate: AtomicF32,
    pub target_factor: AtomicF32,
}

pub static PROCESS_DATA: ProcessData = ProcessData {
    rpm: AtomicF32::new(f32::NAN),
    temperature: AtomicF32::new(f32::NAN),
    bat_current: AtomicF32::new(f32::NAN),
    bat_soc: AtomicF32::new(f32::NAN),
    bat_voltage: AtomicF32::new(f32::NAN),
    input_voltage: AtomicF32::new(f32::NAN),
    field_voltage: AtomicF32::new(f32::NAN),
    field_current: AtomicF32::new(f32::NAN),
    pps_temperature: AtomicF32::new(f32::NAN),
    pps_mode: AtomicU8::new(PpsRunningMode::Unknown as u8),
    ble_rate: AtomicF32::new(0.),
    target_factor: AtomicF32::new(0.),
};

/// Output state of the regulator
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
            "{};{};{};{};{};{};{};{};{};{};{};{}",
            self.rpm.load(Ordering::Relaxed),
            self.target_factor.load(Ordering::Relaxed),
            self.field_current.load(Ordering::Relaxed),
            self.field_voltage.load(Ordering::Relaxed),
            self.bat_current.load(Ordering::Relaxed),
            self.bat_soc.load(Ordering::Relaxed),
            self.bat_voltage.load(Ordering::Relaxed),
            self.input_voltage.load(Ordering::Relaxed),
            self.temperature.load(Ordering::Relaxed),
            self.pps_temperature.load(Ordering::Relaxed),
            self.pps_mode.load(Ordering::Relaxed),
            self.ble_rate.load(Ordering::Relaxed),
        )
    }
}

impl LoggerMeta for ProcessData {
    fn get_meta(&self) -> String<{ LINE_LEN }> {
        format!(
            LINE_LEN; "{};{};{};{};{};{};{};{};{};{};{};{}",
            "RPM",
            "Target",
            "Field Current",
            "Field Voltage",
            "Bat Current",
            "Bat SoC",
            "Bat Voltage",
            "Input Voltage",
            "Temperature",
            "PPS Temperature",
            "PPS Mode",
            "BLE Rate",
        )
        .unwrap()
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

impl LoggerMeta for Setpoint {
    fn get_meta(&self) -> String<{ LINE_LEN }> {
        format!(
            LINE_LEN; "{};{};{};{}", "Field Current Limit", "Field Voltage Limit", "PPS Enabled", "Contactor State"
        )
        .unwrap()
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

/// central event type for the regulator
///
/// Processed by state machine, generated by board::io module
#[allow(unused)]
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RegulatorEvent {
    Ready,
    Rpm(RpmEvent),
    Button(ButtonEvent),
    Temperature(TemperatureEvent),
}

const MAX_EVENTS: usize = 10;

pub type SenderType = Sender<'static, CriticalSectionRawMutex, RegulatorEvent, MAX_EVENTS>;
pub type ReceiverType = Receiver<'static, CriticalSectionRawMutex, RegulatorEvent, MAX_EVENTS>;
type ChannelType = Channel<CriticalSectionRawMutex, RegulatorEvent, MAX_EVENTS>;

pub fn prepare_channel() -> &'static mut ChannelType {
    static EVENT_CHANNEL: StaticCell<ChannelType> = StaticCell::new();
    EVENT_CHANNEL.init(Channel::new())
}