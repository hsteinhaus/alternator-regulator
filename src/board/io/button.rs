use async_button::{Button, ButtonConfig, ButtonEvent as AsyncButtonEvent};
use embassy_futures::select::{select3, Either3};
use esp_hal::gpio::{AnyPin, Input, InputConfig, Pull};

use crate::app::shared::SenderType;
use crate::app::shared::{ButtonEvent, RegulatorEvent};


#[embassy_executor::task]
pub async fn button_task(
    button_resources: ButtonResources<'static>,
    sender: SenderType,
) -> ! {
    let (mut button_left, mut button_center, mut button_right) = button_resources.into_buttons();
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

pub struct ButtonResources<'a> {
    pub button_left: AnyPin<'a>,
    pub button_center: AnyPin<'a>,
    pub button_right: AnyPin<'a>,
}

impl<'a> ButtonResources<'a> {
    pub fn into_buttons(self) -> (Button<Input<'a>>, Button<Input<'a>>, Button<Input<'a>>) {
        let button_left = Input::new(self.button_left, InputConfig::default().with_pull(Pull::Up));
        let button_left = Button::new(button_left, ButtonConfig::default());

        let button_center = Input::new(self.button_center, InputConfig::default().with_pull(Pull::Up));
        let button_center = Button::new(button_center, ButtonConfig::default());

        let button_right = Input::new(self.button_right, InputConfig::default().with_pull(Pull::Up));
        let button_right = Button::new(button_right, ButtonConfig::default());
        (button_left, button_center, button_right)
    }
}
