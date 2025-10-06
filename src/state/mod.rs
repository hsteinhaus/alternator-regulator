use async_button::{Button, ButtonEvent as AsyncButtonEvent};
use defmt::{info, Format};
use embassy_futures::select::{select3, Either3};
use embassy_time::{Duration, Ticker};
use esp_hal::gpio::Input;
use esp_hal::xtensa_lx::_export::critical_section::Mutex;

const BUTTON_LOOP_TIME_MS: u64 = 10;

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
    let mut ticker = Ticker::every(Duration::from_millis(BUTTON_LOOP_TIME_MS));
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
        info!("button_event: {:?}", button_event);
//        BUTTON_EVENT.get_mut().replace(button_event);
        ticker.next().await;
    }
}

