#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

extern crate alloc;

use defmt::info;
use esp_backtrace as _;
use esp_println as _;

use task::ble_scan::ble_scan_task;
use crate::board::startup;
use crate::ui::ui_task;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use crate::board::driver::pcnt::{rpm_task, RPM};
// use esp_alloc::HeapStats;
// use esp_println::println;

mod board;
mod ui;
mod util;
mod task;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let mut res = startup::Resources::initialize();
    // Draw a smiley face with white eyes and a red mouth
    //res.display.draw_smiley().unwrap();

    // Spawn ble_scan::run in a separate Embassy task
    spawner
        .spawn(ble_scan_task(res.wifi_ble.ble_connector))
        .expect("Failed to spawn ble_scan_task");
    spawner.spawn(rpm_task()).expect("Failed to spawn rpm_task");
    spawner.spawn(ui_task(res.display)).expect("Failed to spawn ui_task");

    defmt::info!("Embassy initialized!");
    // res.pps.set_current(0.1).set_voltage(3.3).enable(true);
    loop {
        info!("mainloop");
        // let stats: HeapStats = esp_alloc::HEAP.stats();
        // println!("{}", stats);
        info!(
            "PPS state: mode: {:?}, T: {:?}, Vi: {:?}, Vo: {:?}, Io: {:?}",
            res.pps.get_running_mode(),
            res.pps.get_temperature(),
            res.pps.get_input_voltage(),
            res.pps.get_voltage(),
            res.pps.get_current(),
        );
        info!("rpm: {}", RPM.load(core::sync::atomic::Ordering::SeqCst));
        Timer::after(Duration::from_secs(5)).await;
    }
}
