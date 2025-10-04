#![feature(impl_trait_in_assoc_type)]
#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
extern crate alloc;

use core::ptr::addr_of_mut;
use core::sync::atomic::Ordering;
use defmt::{error, info};
use esp_backtrace as _;
use esp_println as _;
use static_cell::StaticCell;

use embassy_time::{Duration, Timer};
use esp_hal::{
    gpio::Output,
    main,
    system::{Cpu, Stack},
};
use esp_hal_embassy::{Callbacks, Executor};

use board::startup;

use io::{
    ble_scan::ble_scan_task,
};

use ui::ui_task;
use crate::io::{io_task, SETPOINT};

#[allow(dead_code)]
mod board;
mod io;
mod ui;
mod util;
mod control;

static mut APP_CORE_STACK: Stack<8192> = Stack::new();

#[allow(dead_code)]
#[derive(Debug)]
struct CpuLoadHooks {
    core_id: usize,
    led_pin: Output<'static>,
}

impl Callbacks for CpuLoadHooks {
    fn before_poll(&mut self) {
        //self.led_pin.set_high();
    }

    fn on_idle(&mut self) {
        //self.led_pin.set_low();
    }
}

#[main]
#[allow(dead_code)]
fn main() -> ! {
    let mut res = startup::Resources::initialize(); // intentionally non-static, compontents are intended to be moved out into the tasks
    info!("Embassy initialized!");

    // start APP core executor first, as running the PRO core executor will block
    let _guard = res
        .cpu_control
        .start_app_core(unsafe { &mut *addr_of_mut!(APP_CORE_STACK) }, move || {
            static EXECUTOR_APP: StaticCell<Executor> = StaticCell::new();
            let executor_app = EXECUTOR_APP.init(Executor::new());
            executor_app.run_with_callbacks(
                |spawner_app| {
                    spawner_app.spawn(app_main()).ok();
                },CpuLoadHooks {
                    core_id: 1,
                    led_pin: res.led1,
                },
            );
        })
        .unwrap();

    // start PRO core executor
    static EXECUTOR_PRO: StaticCell<::esp_hal_embassy::Executor> = StaticCell::new();
    let executor_pro = EXECUTOR_PRO.init(::esp_hal_embassy::Executor::new());
    executor_pro.run_with_callbacks(
        |spawner_pro| {
            spawner_pro
                .spawn(ble_scan_task(res.wifi_ble.ble_connector))
                .expect("Failed to spawn ble_scan_task");
            spawner_pro
                .spawn(ui_task(res.display))
                .expect("Failed to spawn ui_task");
//            spawner_pro.spawn(adc_task(res.adc)).expect("Failed to spawn adc_task");
            spawner_pro.spawn(io_task(res.adc, res.pcnt, res.pps)).expect("Failed to spawn io_task");
            spawner_pro.spawn(pro_main()).expect("Failed to spawn pro_main");
        }, CpuLoadHooks {
            core_id: 0,
            led_pin: res.led0,
        },
    );
}

#[embassy_executor::task]
async fn app_main() -> ! {
    error!("Starting app_main");
    Timer::after(Duration::from_millis(5000)).await;

    SETPOINT.field_current_limit.store(2.0 , Ordering::SeqCst);
    SETPOINT.field_voltage_limit.store(20.0 , Ordering::SeqCst);
    SETPOINT.pps_enabled.store(true , Ordering::SeqCst);

    loop {
        info!("Hello from core {}", Cpu::current() as usize);
        Timer::after(Duration::from_millis(100)).await;
    }
}

//#[esp_hal_embassy::main]
#[embassy_executor::task]
async fn pro_main() -> () {
    // res.pps.set_current(0.1).set_voltage(3.3).enable(true);
    loop {
        info!("Hello from core {}", Cpu::current() as usize);
        // let stats: HeapStats = esp_alloc::HEAP.stats();
        // println!("{}", stats);
        Timer::after(Duration::from_millis(100)).await;
    }
}
