use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
};
use static_cell::StaticCell;

pub mod regulator_mode;

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
    Rpm(RpmEvent),
    Button(ButtonEvent),
    Temperature(TemperatureEvent),
}

const MAX_EVENTS: usize = 10;

pub type SenderType = Sender<'static, CriticalSectionRawMutex, RegulatorEvent, MAX_EVENTS>;
type ReceiverType = Receiver<'static, CriticalSectionRawMutex, RegulatorEvent, MAX_EVENTS>;
type ChannelType = Channel<CriticalSectionRawMutex, RegulatorEvent, MAX_EVENTS>;

pub fn prepare_channel() -> &'static mut ChannelType {
    static EVENT_CHANNEL: StaticCell<ChannelType> = StaticCell::new();
    EVENT_CHANNEL.init(Channel::new())
}

