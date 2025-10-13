use static_cell::make_static;
use statig::prelude::*;

use super::ReceiverType;
use crate::app::shared::CONTROLLER;
use crate::app::shared::PROCESS_DATA;
use crate::app::state::{ButtonEvent, RegulatorEvent, RpmEvent};

#[derive(Default)]
pub struct RegulatorMode;

#[state_machine(
    initial = "State::off()",
    state(derive(Debug,)),
    superstate(derive(Debug,)),
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
                    CONTROLLER.lock(|c| {
                        c.borrow_mut().adjust_target_inc(0.1*(*count as f32));
                    });
                    Handled
                }
                ButtonEvent::DecShort(count) => {
                    CONTROLLER.lock(|c| {
                        c.borrow_mut().adjust_target_inc(-0.1*(*count as f32));
                    });
                    Handled
                }
                _ => Handled,
            },
            _ => Handled,
        }
    }

    #[action]
    async fn enter_idle(&mut self) {
        debug!("entering idle state");
        CONTROLLER.lock(|c| {
            c.borrow_mut().stop_charging();
        });
    }

    #[action]
    async fn enter_charging(&mut self) {
        debug!("entering charging state");
        CONTROLLER.lock(|c| {
            c.borrow_mut().start_charging();
        });
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
