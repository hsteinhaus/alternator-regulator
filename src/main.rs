#![no_std]
#![no_main]

#![feature(int_from_ascii)]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

extern crate alloc; // MUST be the first module
mod fmt;

mod app;
mod board;
mod ui;
mod util;


use esp_backtrace as _;
use esp_println as _;
use static_cell::make_static;

use board::io::button::button_task;
use board::io::{pps::pps_task, radio::radio_task, rpm::rpm_task};
use board::resources;
use embassy_time::{Duration, Ticker, Timer};
use esp_alloc::HeapStats;

use esp_hal::{
    gpio::Output,
    main,
    system::{CpuControl, Stack},
};
use esp_hal_embassy::{Callbacks, Executor};

use crate::board::io::spi2::spi2_task;
use app::control::controller_task;
use app::mode::regulator_mode_task;
use app::shared::{RegulatorEvent, SenderType};
use fmt::Debug2Format;
use util::led_debug::LedDebug;

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


/// catch-all handler for critical startup errors
#[main]
fn main() -> ! {
    #[cfg(feature = "log-04")]
    {
        use esp_println::logger::init_logger_from_env;
        init_logger_from_env();
    }

    let peripherals = resources::initialize();
    let (led_resources, spi2_resources, pps_resources, button_resources, radio_resources, rpm_resources, system_resources) = resources::collect(peripherals);
    esp_hal_embassy::init(system_resources.timer0);
    info!("Embassy initialized!");

    let leds = led_resources.into_leds();
    let (button_left, button_center, button_right) = button_resources.into_buttons();

    LedDebug::create(leds.user);

    let channel = app::shared::prepare_channel();
    let button_sender = channel.sender();
    let rpm_sender = channel.sender();
    let ready_sender = channel.sender();
    let receiver = channel.receiver();

    // start APP core executor first, as running the PRO core executor will block
    let mut cpu_ctrl = CpuControl::new(system_resources.cpu_ctrl);
    let app_core_stack = make_static!(Stack::<8192>::new());
    let _guard = cpu_ctrl
        .start_app_core(app_core_stack, move || {
            let executor_app = make_static!(Executor::new());
            executor_app.run_with_callbacks(
                |spawner_app| {
                    // spawn FAST tasks on APP core
                    spawner_app.must_spawn(button_task(
                        button_sender,
                        button_left,
                        button_center,
                        button_right,
                    ));
                    spawner_app.must_spawn(rpm_task(rpm_resources, rpm_sender));
                    spawner_app.must_spawn(controller_task());
                    spawner_app.must_spawn(app_main(ready_sender));
                    spawner_app.must_spawn(pps_task(pps_resources));
                    spawner_app.must_spawn(regulator_mode_task(receiver));
                },
                CpuLoadHooks {
                    core_id: 1,
                    led_pin: leds.core1,
                },
            );
        })
        .expect("Critical - failed to start APP core");

    // start PRO core executor
    let executor_pro = make_static!(Executor::new());
    executor_pro.run_with_callbacks(
        |spawner_pro| {
            spawner_pro.must_spawn(radio_task(radio_resources));
            spawner_pro.must_spawn(spi2_task(spawner_pro.clone(), spi2_resources));
            spawner_pro.must_spawn(pro_main());
        },
        CpuLoadHooks {
            core_id: 0,
            led_pin: leds.core0,
        },
    );
}

#[embassy_executor::task]
async fn app_main(ready_sender: SenderType) -> ! {
    info!("Starting app_main");
    ready_sender.send(RegulatorEvent::Ready).await;
    Timer::after(Duration::from_millis(1000)).await;

    let mut ticker = Ticker::every(Duration::from_millis(1000));
    loop {
        // SETPOINT
        //     .field_current_limit
        //     .store(rng.random() as f32 / u32::MAX as f32 * 2., Ordering::Relaxed);
        // SETPOINT
        //     .field_voltage_limit
        //     .store(rng.random() as f32 / u32::MAX as f32 * 20., Ordering::Relaxed);
        // SETPOINT.pps_enabled.store(SetMode::On as u8, Ordering::Relaxed);
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
