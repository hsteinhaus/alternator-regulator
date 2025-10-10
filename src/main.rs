#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

// MUST be the first module
mod fmt;

use core::sync::atomic::Ordering;
use esp_backtrace as _;
use esp_println as _;
use static_cell::{make_static};

use embassy_time::{Duration, Ticker, Timer};
use esp_alloc::HeapStats;
use esp_hal::{
    gpio::Output,
    main,
    system::{Stack},
};
use esp_hal::rng::Rng;
use esp_hal_embassy::{Callbacks, Executor};


use ui::ui_task;
use crate::board::{
    startup,
    driver::pps::SetMode,
};
use crate::io::{io_task, ble_scan::ble_scan_task, rpm::rpm_task, SETPOINT};
use crate::state::regulator_mode::regulator_mode_task;

mod board;
mod io;
mod ui;
mod control;
mod state;
mod util;

#[allow(dead_code)]
#[derive(Debug)]
struct CpuLoadHooks {
    core_id: usize,
    led_pin: Output<'static>,
}

impl Callbacks for CpuLoadHooks {
    fn before_poll(&mut self) {
        self.led_pin.set_high();
    }

    fn on_idle(&mut self) {
        self.led_pin.set_low();
    }
}

#[main]
#[allow(dead_code)]
fn main() -> ! {
    #[cfg(feature = "log-04")]
    esp_println::logger::init_logger(log_04::LevelFilter::Info);

    let mut res = startup::Resources::initialize(); // intentionally non-static, compontents are intended to be moved out into the tasks
    info!("Embassy initialized!");

    let channel = state::prepare_channel();
    let button_sender = channel.sender();
    let rpm_sender = channel.sender();
    let receiver = channel.receiver();

    // start APP core executor first, as running the PRO core executor will block
    let app_core_stack = make_static!(Stack::<8192>::new());
    let _guard = res
        .cpu_control
        .start_app_core(app_core_stack, move || {
            let executor_app = make_static!(Executor::new());
            executor_app.run_with_callbacks(
                |spawner_app| {
                    // spawn FAST tasks on APP core
                    spawner_app.must_spawn(state::button_task(button_sender, res.button_left, res.button_center, res.button_right));
                    spawner_app.must_spawn(rpm_task(rpm_sender, res.pcnt));
                    spawner_app.must_spawn(app_main(res.rng));
                    spawner_app.must_spawn(regulator_mode_task(receiver));
                },CpuLoadHooks {
                    core_id: 1,
                    led_pin: res.led1,
                },
            );
        })
        .unwrap();

    // start PRO core executor
    let executor_pro = make_static!(Executor::new());
    executor_pro.run_with_callbacks(
        |spawner_pro| {
            spawner_pro.must_spawn(ble_scan_task(res.wifi_ble.ble_connector));
            spawner_pro.must_spawn(ui_task(res.display));
            spawner_pro.must_spawn(io_task(res.adc, res.pps));
//            spawner_pro.must_spawn(state::state_task(receiver));
            spawner_pro.must_spawn(pro_main());
        }, CpuLoadHooks {
            core_id: 0,
            led_pin: res.led0,
        },
    );
}


#[embassy_executor::task]
async fn app_main(mut rng: Rng) -> ! {
    info!("Starting app_main");
    Timer::after(Duration::from_millis(5050)).await;

    let mut ticker = Ticker::every(Duration::from_millis(1000));
    loop {
        SETPOINT.field_current_limit.store(rng.random() as f32/u32::MAX as f32 * 2. , Ordering::SeqCst);
        SETPOINT.field_voltage_limit.store(rng.random() as f32/u32::MAX as f32 * 20. , Ordering::SeqCst);
        SETPOINT.pps_enabled.store(SetMode::On as u8 , Ordering::SeqCst);
        ticker.next().await;
    }
}

#[embassy_executor::task]
async fn pro_main() -> () {
    info!("Starting pro_main");
    let mut ticker = Ticker::every(Duration::from_millis(60_000));
    loop {
        let stats: HeapStats = esp_alloc::HEAP.stats();
        info!("{}", stats);
        ticker.next().await;
    }
}
