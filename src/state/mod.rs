use async_button::{Button, ButtonEvent as AsyncButtonEvent};
use static_cell::StaticCell;
use defmt::{debug, Format};
use embassy_futures::select::{select3, Either3};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
};
use embassy_time::{Duration, Ticker};
use esp_hal::gpio::Input;

#[derive(Copy, Clone, Debug, Format)]
pub enum ButtonEvent {
    DecShort(usize),
    DecLong,
    OkShort(usize),
    OkLong,
    IncShort(usize),
    IncLong,
}

const MAX_EVENTS: usize = 10;
type ChannelType = Channel<CriticalSectionRawMutex, ButtonEvent, MAX_EVENTS>;


pub fn prepare_channel() -> &'static mut ChannelType {
    static EVENT_CHANNEL: StaticCell<ChannelType> = StaticCell::new();
    EVENT_CHANNEL.init(Channel::new())
}

#[embassy_executor::task]
pub async fn state_task(receiver: Receiver<'static, CriticalSectionRawMutex, ButtonEvent, 10>) -> ! {
    let mut ticker = Ticker::every(Duration::from_millis(100));
    loop {
        let evt = receiver.receive().await;
        debug!("received button evt: {:?}", evt);
        ticker.next().await;
    }
}

#[embassy_executor::task]
pub async fn button_task(
    sender: Sender<'static, CriticalSectionRawMutex, ButtonEvent, 10>,
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
        sender.send(button_event).await;
        // intentionally no timer/ticker here, loop is inhibited by polling the update() method of the buttons
    }
}
