use esp_hal::{
    peripherals::{BT, WIFI},
    rng::Rng,
    timer::AnyTimer,
};
use esp_wifi::ble::controller::BleConnector;
use esp_wifi::{EspWifiController, InitializationError};
use static_cell::make_static;
use thiserror_no_std::Error;

#[derive(Debug, Error)]
pub enum WifiError {
    #[error("Failed to initialize WIFI/BLE controller")]
    WifiInitError(#[from] InitializationError),

    #[error("Failed to initialize WIFI controller")]
    WifiControllerError(#[from] esp_wifi::wifi::WifiError),
}

#[allow(dead_code)]
pub struct WifiDriver {
    pub wifi_init: &'static EspWifiController<'static>,
    pub ble_connector: BleConnector<'static>,
}

impl WifiDriver {
    pub fn new(wifi: WIFI<'static>, bt: BT<'static>, timer: AnyTimer<'static>, rng: Rng) -> Result<Self, WifiError> {
        let wifi_init = make_static!(esp_wifi::init(timer, rng)?);
        let (_wifi_controller, _interfaces) = esp_wifi::wifi::new(wifi_init, wifi)?;
        let ble_connector = BleConnector::new(wifi_init, bt);

        Ok(Self {
            wifi_init,
            ble_connector,
        })
    }
}
