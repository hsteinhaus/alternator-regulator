use super::ReceiverType;
use crate::io::PROCESS_DATA;
use crate::state::{ButtonEvent, RegulatorEvent, RpmEvent};
use defmt::{debug, trace, Format};
use static_cell::make_static;
use statig::prelude::*;
use crate::control::{dec_current, inc_current, start_charging, stop_charging};

#[derive(Default)]
pub struct RegulatorMode;

#[state_machine(
    initial = "State::off()",
    state(derive(Debug, Format)),
    superstate(derive(Debug, Format)),
    before_transition = "Self::before_transition"
)]
impl RegulatorMode {
    #[state]
    async fn off(event: &RegulatorEvent) -> Outcome<State> {
        match event {
            RegulatorEvent::Button(button) => match button {
                ButtonEvent::OkLong => {
                    if PROCESS_DATA.rpm_is_normal() {
                        Transition(State::charging())
                    } else {
                        Transition(State::idle())
                    }
                }
                _ => Handled,
            },
            _ => Handled,
        }
    }

    #[state(entry_action = "enter_idle")]
    async fn idle(event: &RegulatorEvent) -> Outcome<State> {
        match event {
            RegulatorEvent::Rpm(rpm) => match rpm {
                RpmEvent::Normal => Transition(State::charging()),
                _ => Handled,
            },
            _ => Handled,
        }
    }

    #[state(entry_action = "enter_charging")]
    async fn charging(event: &RegulatorEvent) -> Outcome<State> {
        match event {
            RegulatorEvent::Rpm(rpm) => match rpm {
                RpmEvent::Low => Transition(State::idle()),
                _ => Handled,
            },
            RegulatorEvent::Button(button) => match button {
                ButtonEvent::IncShort(count) => {
                    inc_current(*count as u32);
                    Handled
                }
                ButtonEvent::DecShort(count) => {
                    dec_current(*count as u32);
                    Handled
                }
                _ => Handled,
            }
            _ => Handled,
        }
    }

    #[action]
    async fn enter_idle(&mut self) {
        debug!("entering idle state");
        stop_charging();
    }

    #[action]
    async fn enter_charging(&mut self) {
        debug!("entering charging state");
        start_charging();
    }
}

impl RegulatorMode {
    async fn before_transition(&mut self, source: &State, target: &State, _context: &mut ()) {
        trace!("before transitioned from `{:?}` to `{:?}`", source, target);
    }
}

#[embassy_executor::task]
pub async fn regulator_mode_task(receiver: ReceiverType) -> ! {
    let state_machine = make_static!(RegulatorMode::default().state_machine());
    loop {
        let evt = receiver.receive().await;
        debug!("received event: {:?}", evt);
        state_machine.handle(&evt).await;
        // intentionally no timer/ticker here, loop is inhibited by receive() and handle()
    }
}
