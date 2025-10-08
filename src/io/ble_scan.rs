use crate::io::PROCESS_DATA;
use bt_hci::cmd::le::LeSetScanParams;
use bt_hci::controller::ControllerCmdSync;
use bt_hci::event::Vendor;
use defmt::{debug, error, info, warn, Debug2Format};
use embassy_futures::join::join;
use embassy_time::{Duration, Timer};
use trouble_host::prelude::*;
use victron_ble::DeviceState;
use crate::board::driver::wifi_ble::BleControllerType;

/// Max number of connections
const CONNECTIONS_MAX: usize = 3;
const L2CAP_CHANNELS_MAX: usize = 3;
const BT_SCAN_INTERVAL: u64 = 500;
const BT_SCAN_WINDOW: u64 = 400;

const VICTRON_ID: u16 = 0x02e1;
// SmartShunt 500A (Sprinter)
// const VICTRON_KEY: &[u8] = [0x13_u8, 0xc6_u8, 0xbf_u8, 0xf8_u8, 0xdb_u8, 0xef_u8, 0xcf_u8, 0x2d_u8, 0xd5_u8, 0xd5_u8, 0x07_u8, 0x79_u8, 0x8d_u8, 0xc1_u8, 0x0f_u8, 0x9e_u8].as_slice();
// const MAC: [u8; 6] = [0xd9_u8, 0xd5_u8, 0x51_u8, 0x59_u8, 0x70_u8, 0x4d_u8];
// Lader
const VICTRON_KEY: &[u8] = [
    0x34_u8, 0xa4_u8, 0x20_u8, 0xf8_u8, 0x6f_u8, 0xa0_u8, 0x37_u8, 0x50_u8,
    0x8a_u8, 0x83_u8, 0x47_u8, 0xf6_u8, 0x21_u8, 0x4d_u8, 0xc1_u8, 0xf4_u8,
]
.as_slice();
const MAC: [u8; 6] = [0xc0_u8, 0x12_u8, 0x9b_u8, 0x97_u8, 0x7f_u8, 0xb8_u8];
const REV_MAC: [u8; 6] = [MAC[5], MAC[4], MAC[3], MAC[2], MAC[1], MAC[0]];

async fn run<C>(controller: C)
where
    C: Controller + ControllerCmdSync<LeSetScanParams>,
{
    // Using a fixed "random" address can be useful for testing. In real scenarios, one would
    // use e.g. the MAC 6 byte array as the address (how to get that varies by the platform).
    let address: Address = Address::random([0xff, 0x8f, 0x1b, 0x05, 0xe4, 0xff]);
    let rev_mac_addr = BdAddr::new(REV_MAC);
    info!("Our address = {:?}", address);

    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);

    let Host {
        central, mut runner, ..
    } = stack.build();

    let printer = VictronBLE {
        paired_key: VICTRON_KEY,
        paired_mac: BdAddr::new(REV_MAC),
    };

    let mut scanner = Scanner::new(central);
    let _ = join(runner.run_with_handler(&printer), async {
        let filter = [(AddrKind::RANDOM, &rev_mac_addr)];
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
                error!("scan error: {:?}", Debug2Format(&e));
                break;
            }
            Timer::after(Duration::from_millis(BT_SCAN_INTERVAL)).await;
        }
    })
    .await;
}

struct VictronBLE {
    paired_mac: BdAddr,
    paired_key: &'static [u8],
}

impl VictronBLE {
    fn handle_mdata(&self, data: &[u8]) {
        let device_state_result = victron_ble::parse_manufacturer_data(data, self.paired_key);
        match device_state_result {
            Ok(device_state) => {
                match device_state {
                    DeviceState::GridCharger(gc_state) => {
                        // just using this AC charger to bring BLE test data into the system
                        // fake a current reading from normally non-zero voltage
                        PROCESS_DATA
                            .bat_current
                            .store(gc_state.battery_voltage1_v, core::sync::atomic::Ordering::SeqCst);
                    }
                    DeviceState::BatteryMonitor(bm_state) => {
                        PROCESS_DATA
                            .bat_voltage
                            .store(bm_state.battery_current_a, core::sync::atomic::Ordering::SeqCst);
                        PROCESS_DATA
                            .bat_current
                            .store(bm_state.battery_current_a, core::sync::atomic::Ordering::SeqCst);
                        PROCESS_DATA
                            .soc
                            .store(bm_state.state_of_charge_pct, core::sync::atomic::Ordering::SeqCst);
                    }
                    _ => {}
                }
                debug!("Victron Data: {:?}", Debug2Format(&device_state));
            }
            Err(e) => {
                warn!("Victron Data Error: {:?}", Debug2Format(&e));
            }
        }
    }
}

impl EventHandler for VictronBLE {
    fn on_vendor(&self, vendor: &Vendor) {
        info!("vendor: {:?}", vendor);
    }

    fn on_adv_reports(&self, mut it: LeAdvReportsIter<'_>) {
        while let Some(Ok(report)) = it.next() {
            if report.addr != self.paired_mac {
                warn!(
                    "ignoring {:x}, that has unexpectedly passed the scan filter",
                    report.addr
                );
                continue;
            };
            for ad in AdStructure::decode(report.data) {
                match ad {
                    Ok(ad) => {
                        match ad {
                            AdStructure::ManufacturerSpecificData {
                                company_identifier,
                                payload,
                            } => {
                                if company_identifier == VICTRON_ID {
                                    self.handle_mdata(payload);
                                    //warn!("Victron ad: {:?}", payload);
                                } else {
                                    warn!("ignoring non-Victron ad: {:?}", company_identifier);
                                }
                            }
                            _ => continue,
                        }
                    }
                    Err(_) => {
                        warn!("ad decode error");
                        continue;
                    }
                };
            }
        }
    }

    fn on_ext_adv_reports(&self, _reports: LeExtAdvReportsIter) {
        warn!("ext adv reports");
    }
}

#[embassy_executor::task]
pub async fn ble_scan_task(controller: BleControllerType) {
    run(controller).await;
}
