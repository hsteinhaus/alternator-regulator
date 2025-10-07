#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
extern crate alloc;

use core::sync::atomic::Ordering;
use defmt::{info};
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

use board::startup;

use io::{
    ble_scan::ble_scan_task,
};

use ui::ui_task;
use crate::board::driver::pps::SetMode;
use crate::io::{io_task, rpm_task, SETPOINT};
use crate::state::button_task;

#[allow(dead_code)]
mod board;
mod io;
mod ui;
mod control;
mod state;


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
    let mut res = startup::Resources::initialize(); // intentionally non-static, compontents are intended to be moved out into the tasks
    info!("Embassy initialized!");

    // start APP core executor first, as running the PRO core executor will block
    let app_core_stack = make_static!(Stack::<8192>::new());
    let _guard = res
        .cpu_control
        .start_app_core(app_core_stack, move || {
            let executor_app = make_static!(Executor::new());
            executor_app.run_with_callbacks(
                |spawner_app| {
                    // spawn FAST tasks on APP core
                    spawner_app.spawn(button_task(res.button_left, res.button_center, res.button_right)).expect("Failed to spawn button_task");
                    spawner_app.spawn(rpm_task(res.pcnt)).expect("Failed to spawn rpm_task");
                    spawner_app.spawn(app_main(res.rng)).ok();
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
            spawner_pro
                // spawn SLOW tasks on APP core
                .spawn(ble_scan_task(res.wifi_ble.ble_connector))
                .expect("Failed to spawn ble_scan_task");
            spawner_pro
                .spawn(ui_task(res.display))
                .expect("Failed to spawn ui_task");
            spawner_pro.spawn(io_task(res.adc, res.pps)).expect("Failed to spawn io_task");
            spawner_pro.spawn(pro_main()).expect("Failed to spawn pro_main");
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
