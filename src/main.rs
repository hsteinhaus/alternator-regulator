#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

extern crate alloc;

use defmt;
use defmt::info;
use esp_backtrace as _;
use esp_println as _;

use crate::ble_scan::ble_scan_task;
use crate::board::startup;
use crate::ui::ui_task;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_alloc::HeapStats;
use esp_println::println;

mod ble_scan;
mod board;
mod ui;
mod util;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let res = startup::Resources::initialize();
    // Draw a smiley face with white eyes and a red mouth
    //res.display.draw_smiley().unwrap();

    // Spawn ble_scan::run in a separate Embassy task
    spawner
        .spawn(ble_scan_task(res.wifi_ble.ble_connector))
        .expect("Failed to spawn ble_scan_task");

    spawner.spawn(ui_task(res.display)).expect("Failed to spawn ui_task");

    defmt::info!("Embassy initialized!");
    loop {
        let stats: HeapStats = esp_alloc::HEAP.stats();
        info!("mainloop");
        println!("{}", stats);
        Timer::after(Duration::from_secs(5)).await;
    }
}
