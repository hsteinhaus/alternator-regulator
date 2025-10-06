use async_button::{Button, ButtonEvent as AsyncButtonEvent};
use defmt::{warn, Format};
use embassy_futures::select::{select3, Either3};
use esp_hal::gpio::Input;


#[derive(Debug, Format)]
enum ButtonEvent {
    DecShort(usize),
    DecLong,
    OkShort(usize),
    OkLong,
    IncShort(usize),
    IncLong,
}

//static mut BUTTON_EVENT: Mutex<Option<ButtonEvent>> = Mutex::new(None);


#[embassy_executor::task]
pub async fn button_task(mut button_left: Button<Input<'static>>, mut button_center: Button<Input<'static>>, mut button_right: Button<Input<'static>>) -> !{
    loop {
        // decode button events
        let button_event = match select3(button_left.update(), button_center.update(), button_right.update()).await {
            Either3::First(event) => {
                match event {
                    AsyncButtonEvent::ShortPress { count } => ButtonEvent::DecShort(count),
                    AsyncButtonEvent::LongPress => ButtonEvent::DecLong,
                }
            }
            Either3::Second(event) => {
                match event {
                    AsyncButtonEvent::ShortPress { count } => ButtonEvent::OkShort(count),
                    AsyncButtonEvent::LongPress => ButtonEvent::OkLong,
                }

            }
            Either3::Third(event) => {
                match event {
                    AsyncButtonEvent::ShortPress { count } => ButtonEvent::IncShort(count),
                    AsyncButtonEvent::LongPress => ButtonEvent::IncLong,
                }
            }
        };
        warn!("button_event: {:?}", button_event);
//        BUTTON_EVENT.get_mut().replace(button_event);
        // intentionally no timer/ticker here, loop is inhibited by polling the update() method of the buttons
    }
}

