use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
};
use heapless::{format, String};
use libm::{fmaxf, fminf};
use static_cell::{make_static, StaticCell};
use statig::prelude::*;

use crate::app::control::Controller;
use crate::app::shared::{ButtonEvent, RegulatorEvent, RpmEvent, CONTROLLER, REGULATOR_MODE, RM_LEN};

const MAX_EVENTS: usize = 10;

pub type SenderType = Sender<'static, CriticalSectionRawMutex, RegulatorEvent, MAX_EVENTS>;
type ReceiverType = Receiver<'static, CriticalSectionRawMutex, RegulatorEvent, MAX_EVENTS>;
type ChannelType = Channel<CriticalSectionRawMutex, RegulatorEvent, MAX_EVENTS>;

pub fn prepare_channel() -> &'static mut ChannelType {
    static EVENT_CHANNEL: StaticCell<ChannelType> = StaticCell::new();
    EVENT_CHANNEL.init(Channel::new())
}

#[derive(Default, Debug)]
pub struct RegulatorMode;

#[state_machine(
    initial = "State::startup()",
    state(derive(Debug,)),
    superstate(derive(Debug,)),
    after_transition = "Self::after_transition"
)]
impl RegulatorMode {
    const DUMMY_STR: String<RM_LEN> = String::new();

    #[state]
    async fn startup(event: &RegulatorEvent) -> Outcome<State> {
        match event {
            RegulatorEvent::Ready => Transition(State::off()),
            _ => Handled,
        }
    }

    /// Off state - no field current, no RPM measurement possible
    #[state(entry_action = "enter_off")]
    async fn off(event: &RegulatorEvent) -> Outcome<State> {
        match event {
            RegulatorEvent::Button(button) => match button {
                ButtonEvent::OkLong => Transition(State::idle()),
                _ => Handled,
            },
            _ => Handled,
        }
    }

    /// Idle state - field current is set to 1.0A to allow for RPM measurement is possible
    /// field current is not controlled, no significant charging current
    #[state(entry_action = "enter_idle")]
    async fn idle(event: &RegulatorEvent) -> Outcome<State> {
        match event {
            RegulatorEvent::Rpm(rpm) => match rpm {
                RpmEvent::Normal => Transition(State::charging()),
                _ => Handled,
            },
            RegulatorEvent::Button(button) => match button {
                ButtonEvent::IncLong => Transition(State::charging()),
                ButtonEvent::OkShort(_) => Transition(State::off()),
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
                        c.adjust_target_factor_inc(fminf(0.05 * (*count as f32), 1.));
                    });
                    Handled
                }

                ButtonEvent::DecShort(count) => {
                    CONTROLLER.lock(|c| {
                        let c: &mut Controller = &mut c.borrow_mut();
                        c.adjust_target_factor_inc(fmaxf(-0.05 * (*count as f32), -1.));
                    });
                    Handled
                }
                ButtonEvent::DecLong => Transition(State::idle()),
                ButtonEvent::OkShort(_) => Transition(State::off()),
                _ => Handled,
            },
            _ => Handled,
        }
    }

    #[action]
    async fn enter_idle(&mut self) {
        info!("entering idle state");
        CONTROLLER.lock(|c| {
            let c: &mut Controller = &mut c.borrow_mut();
            c.start_idle();
        });
    }

    #[action]
    async fn enter_charging(&mut self) {
        info!("entering charging state");
        CONTROLLER.lock(|c| {
            let c: &mut Controller = &mut c.borrow_mut();
            c.start_charging();
        });
    }

    #[action]
    async fn enter_off(&mut self) {
        info!("entering off state");
        CONTROLLER.lock(|c| {
            let c: &mut Controller = &mut c.borrow_mut();
            c.stop();
        });
    }
}

impl RegulatorMode {
    async fn after_transition(&mut self, source: &State, target: &State, _context: &mut ()) {
        trace!("after_transition: {:?} -> {:?}", source, target);
        let state_name = format!(RM_LEN; "{:?}", target).unwrap_or_else(|_| Self::DUMMY_STR);
        //debug!("after_transition: {:?}", state_name);
        // debug!(
        //     "after_transition: {:?} -> {:?} -> {:?}",
        //     source, target, state_name
        // );
        REGULATOR_MODE.lock(|rm| {
            let rm: &mut String<RM_LEN> = &mut rm.borrow_mut();
            rm.clear();
            rm.push_str(&state_name).unwrap_or_else(|_|());  // should never fail, as both strings are of RM_LEN
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
