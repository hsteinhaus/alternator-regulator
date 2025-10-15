use heapless::{format, String};
use libm::{fmaxf, fmin, fminf};
use static_cell::make_static;
use statig::prelude::*;
use crate::app::control::Controller;
use super::ReceiverType;
use crate::app::shared::{CONTROLLER, REGULATOR_MODE, RM_LEN};
use crate::app::shared::PROCESS_DATA;
use crate::app::statemachine::{ButtonEvent, RegulatorEvent, RpmEvent};

#[derive(Default)]
pub struct RegulatorMode;

#[state_machine(
    initial = "State::startup()",
    state(derive(Debug,)),
    superstate(derive(Debug,)),
    after_transition = "Self::after_transition"
)]
impl RegulatorMode {
    #[state]
    async fn startup(event: &RegulatorEvent) -> Outcome<State> {
        match event {
            RegulatorEvent::Ready => Transition(State::off()),
            _ => Handled,
        }
    }


    #[state(entry_action = "enter_off")]
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
                        let c: &mut Controller = &mut c.borrow_mut();
                        c.adjust_target_inc(fminf(0.1*(*count as f32), 0.));
                    });
                    Handled
                }

                ButtonEvent::DecShort(count) => {
                    CONTROLLER.lock(|c| {
                        let c: &mut Controller = &mut c.borrow_mut();
                        c.adjust_target_inc(fmaxf(-0.1*(*count as f32), 0.));
                    });
                    Handled
                }
                ButtonEvent::OkShort(_) => Transition(State::off()),
                ButtonEvent::OkLong => Transition(State::off()),
                _ => Handled,
            },
            _ => Handled,
        }
    }

    #[action]
    async fn enter_idle(&mut self) {
        debug!("entering idle state");
        CONTROLLER.lock(|c| {
            let c: &mut Controller = &mut c.borrow_mut();
            c.stop_charging();
        });
    }

    #[action]
    async fn enter_charging(&mut self) {
        debug!("entering charging state");
        CONTROLLER.lock(|c| {
            let c: &mut Controller = &mut c.borrow_mut();
            c.start_charging();
        });
    }

    #[action]
    async fn enter_off(&mut self) {
        debug!("entering off state");
        CONTROLLER.lock(|c| {
            let c: &mut Controller = &mut c.borrow_mut();
            c.stop_charging();
        });
    }
}

impl RegulatorMode {
    async fn after_transition(&mut self, source: &State, target: &State, _context: &mut ()) {
        trace!("after_transition: {:?} -> {:?}", source, target);
        let state_name: String<RM_LEN> = format!("{:?}", target).unwrap();
        REGULATOR_MODE.lock(|rm| {
            let rm: &mut String<RM_LEN> = &mut rm.borrow_mut();
            rm.clear();
            rm.push_str(&state_name).unwrap();
        });
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
