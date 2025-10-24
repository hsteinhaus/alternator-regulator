use bt_hci::cmd::le::LeSetScanParams;
use bt_hci::controller::ControllerCmdSync;
use embassy_futures::join::join;
use embassy_time::{Duration, Timer};
use esp_hal::{
    peripherals::{BT, WIFI},
    rng::Rng,
    timer::AnyTimer,
};
use trouble_host::prelude::*;

use crate::app::victron::VictronBLE;
use crate::board::driver::radio::{WifiDriver, WifiError};

/// Max number of connections
const CONNECTIONS_MAX: usize = 3;
const L2CAP_CHANNELS_MAX: usize = 3;
const BT_SCAN_INTERVAL: u64 = 500;
const BT_SCAN_WINDOW: u64 = 400;

async fn run<C>(controller: C)
where
    C: Controller + ControllerCmdSync<LeSetScanParams>,
{
    // Using a fixed "random" address can be useful for testing. In real scenarios, one would
    // use e.g. the MAC 6 byte array as the address (how to get that varies by the platform).
    let address: Address = Address::random([0xff, 0x8f, 0x1b, 0x05, 0xe4, 0xff]);
    info!("Our address = {:?}", address);

    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);

    let Host {
        central, mut runner, ..
    } = stack.build();

    let handler = VictronBLE::new();
    let mut scanner = Scanner::new(central);
    let _ = join(runner.run_with_handler(&handler), async {
        let filter = [(AddrKind::RANDOM, &handler.paired_mac)];
        let config = ScanConfig {
            active: false,
            interval: Duration::from_millis(BT_SCAN_INTERVAL),
            window: Duration::from_millis(BT_SCAN_WINDOW),
            filter_accept_list: &filter,
            ..Default::default()
        };

        // Scan forever
        loop {
            let res = scanner.scan(&config).await;
            if let Err(e) = res {
                error!("scan error: {:?}", crate::fmt::Debug2Format(&e));
                break;
            }
            Timer::after(Duration::from_millis(BT_SCAN_INTERVAL)).await;
        }
    })
    .await;
}

#[embassy_executor::task]
pub async fn radio_task(radio_resources: RadioResources<'static>) -> () {
    let driver = match radio_resources.into_driver() {
        Ok(driver) => driver,
        Err(err) => {
            error!("critical error - WIFI startup failed: {:?}", crate::fmt::Debug2Format(&err));
            return;
        }
    };
    let controller = ExternalController::<_, 20>::new(driver.ble_connector);
    run(controller).await;
}

pub struct RadioResources<'a> {
    pub rng: Rng,
    pub wifi: WIFI<'a>,
    pub bt: BT<'a>,
    pub timer: AnyTimer<'a>,
}

impl RadioResources<'static> {
    pub fn into_driver(self) -> Result<WifiDriver, WifiError> {
        Ok(WifiDriver::new(self.wifi, self.bt, self.timer, self.rng)?)
    }
}
