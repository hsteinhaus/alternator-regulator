use embassy_time::{Duration, Ticker};
use statig::prelude::*;
use crate::state::RegulatorEvent;

#[derive(Default)]
pub struct RegulatorMode;


#[state_machine(initial = "State::idle()")]
impl RegulatorMode {
    #[state]
    async fn off(event: &RegulatorEvent) -> Outcome<State> {
        match event {
            RegulatorEvent::Button(_b) => Transition(State::idle()),
            _ => Super
        }
    }

    #[state]
    async fn idle(event: &RegulatorEvent) -> Outcome<State> {
        match event {
            RegulatorEvent::Button(_b) => Transition(State::idle()),
            _ => Super
        }
    }

    #[state]
    async fn charging(event: &RegulatorEvent) -> Outcome<State> {
        match event {
            RegulatorEvent::Button(_b) => Transition(State::idle()),
            _ => Super
        }
    }
}

const IO_LOOP_TIME_MS: u64 = 100;

#[embassy_executor::task]
pub async fn regulator_mode_task() -> ! {
    let mut _state_machine = RegulatorMode::default().state_machine();
    let mut ticker = Ticker::every(Duration::from_millis(IO_LOOP_TIME_MS));
    loop {
        //state_machine.handle(&RegulatorEvent::TimerElapsed).await;
        ticker.next().await;
    }
//    state_machine.handle(&Event::ButtonPressed);
}