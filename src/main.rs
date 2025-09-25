#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

extern crate alloc;

use defmt;
use esp_backtrace as _;
use esp_println as _;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use crate::ble_scan::ble_scan_task;
use crate::board::startup;
use crate::ui::ui_task;

mod ble_scan;
mod board;
mod ui;
mod util;

#[allow(dead_code)]
mod lvgl;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let mut res = startup::Resources::initialize();
    // Draw a smiley face with white eyes and a red mouth
    res.display.draw_smiley().unwrap();

    // Spawn ble_scan::run in a separate Embassy task
    spawner
        .spawn(ble_scan_task(res.wifi_ble.ble_connector))
        .expect("Failed to spawn ble_scan_task");

    spawner.spawn(ui_task(res.display));

    defmt::info!("Embassy initialized!");
    loop {
        defmt::info!("Hello world!");
        Timer::after(Duration::from_secs(1)).await;
    }
}
