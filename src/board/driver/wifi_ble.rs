use bt_hci::controller::ExternalController;
use esp_hal::{
    peripherals::{BT, WIFI},
};
use esp_radio::ble::controller::BleConnector;
use static_cell::{make_static};

pub type BleControllerType = ExternalController<BleConnector<'static>, 20>;

#[allow(dead_code)]
pub struct WifiDriver {
    pub radio: &'static esp_radio::Controller<'static>,
    pub ble_controller: BleControllerType,
}

impl WifiDriver {
    pub fn new(_wifi: WIFI<'static>, bt: BT<'static>/*, timer: AnyTimer<'static>, rng: Rng*/) -> Self {
        let radio = make_static!(esp_radio::init().unwrap());
        let ble_connector = BleConnector::new(radio, bt, Default::default()).unwrap();
        let ble_controller: BleControllerType = ExternalController::new(ble_connector);

        Self {
            radio,
            ble_controller,
        }
    }
}
