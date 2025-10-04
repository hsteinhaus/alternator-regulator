use crate::mk_static;
use esp_hal::{
    peripherals::{BT, WIFI},
    rng::Rng,
    timer::AnyTimer,
};
use esp_wifi::ble::controller::BleConnector;
use esp_wifi::EspWifiController;

#[allow(dead_code)]
pub struct WifiDriver {
    pub wifi_init: &'static EspWifiController<'static>,
    pub ble_connector: BleConnector<'static>,
}

impl WifiDriver {
    pub fn new(wifi: WIFI<'static>, bt: BT<'static>, timer: AnyTimer<'static>, rng: Rng) -> Self {
        let wifi_init = mk_static!(
            EspWifiController,
            esp_wifi::init(timer, rng).expect("Failed to initialize WIFI/BLE controller")
        );
        let (_wifi_controller, _interfaces) =
            esp_wifi::wifi::new(wifi_init, wifi).expect("Failed to initialize WIFI controller");
        let ble_connector = BleConnector::new(wifi_init, bt);

        Self {
            wifi_init,
            ble_connector,
        }
    }
}
