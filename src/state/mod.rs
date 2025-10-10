use async_button::{Button, ButtonEvent as AsyncButtonEvent};
use embassy_futures::select::{select3, Either3};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
};
use esp_hal::gpio::Input;
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

#[embassy_executor::task]
pub async fn button_task(
    sender: SenderType,
    mut button_left: Button<Input<'static>>,
    mut button_center: Button<Input<'static>>,
    mut button_right: Button<Input<'static>>,
) -> ! {
    loop {
        // decode button events
        let button_event = match select3(button_left.update(), button_center.update(), button_right.update()).await {
            Either3::First(event) => match event {
                AsyncButtonEvent::ShortPress { count } => ButtonEvent::DecShort(count),
                AsyncButtonEvent::LongPress => ButtonEvent::DecLong,
            },
            Either3::Second(event) => match event {
                AsyncButtonEvent::ShortPress { count } => ButtonEvent::OkShort(count),
                AsyncButtonEvent::LongPress => ButtonEvent::OkLong,
            },
            Either3::Third(event) => match event {
                AsyncButtonEvent::ShortPress { count } => ButtonEvent::IncShort(count),
                AsyncButtonEvent::LongPress => ButtonEvent::IncLong,
            },
        };
        sender.send(RegulatorEvent::Button(button_event)).await;
        // intentionally no timer/ticker here, loop is inhibited by polling the update() method of the buttons
    }
}
