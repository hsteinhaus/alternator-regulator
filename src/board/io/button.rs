use async_button::{Button, ButtonEvent as AsyncButtonEvent};
use embassy_futures::select::{select3, Either3};
use esp_hal::gpio::Input;
use crate::app::shared::{ButtonEvent, RegulatorEvent};
use crate::app::shared::SenderType;

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
