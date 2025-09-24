pub mod startup;
pub mod driver;

esp_bootloader_esp_idf::esp_app_desc!();

#[no_mangle]
pub extern "Rust" fn _esp_println_timestamp() -> u64 {
    esp_hal::time::Instant::now()
        .duration_since_epoch()
        .as_millis()
}
