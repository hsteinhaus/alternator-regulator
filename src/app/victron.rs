use bt_hci::param::{BdAddr, LeAdvReportsIter, LeExtAdvReportsIter};
use trouble_host::prelude::EventHandler;
use bt_hci::event::Vendor;
use trouble_host::advertise::AdStructure;
use victron_ble::DeviceState;
use crate::app::shared::PROCESS_DATA;

const VICTRON_ID: u16 = 0x02e1;
// SmartShunt 500A (Sprinter)
// const VICTRON_KEY: &[u8] = [0x13_u8, 0xc6_u8, 0xbf_u8, 0xf8_u8, 0xdb_u8, 0xef_u8, 0xcf_u8, 0x2d_u8, 0xd5_u8, 0xd5_u8, 0x07_u8, 0x79_u8, 0x8d_u8, 0xc1_u8, 0x0f_u8, 0x9e_u8].as_slice();
// const MAC: [u8; 6] = [0xd9_u8, 0xd5_u8, 0x51_u8, 0x59_u8, 0x70_u8, 0x4d_u8];
// Lader
pub const VICTRON_KEY: &[u8] = [
    0x34_u8, 0xa4_u8, 0x20_u8, 0xf8_u8, 0x6f_u8, 0xa0_u8, 0x37_u8, 0x50_u8,
    0x8a_u8, 0x83_u8, 0x47_u8, 0xf6_u8, 0x21_u8, 0x4d_u8, 0xc1_u8, 0xf4_u8,
]
.as_slice();
const MAC: [u8; 6] = [0xc0_u8, 0x12_u8, 0x9b_u8, 0x97_u8, 0x7f_u8, 0xb8_u8];
pub const REV_MAC: [u8; 6] = [MAC[5], MAC[4], MAC[3], MAC[2], MAC[1], MAC[0]];

pub struct VictronBLE {
    paired_mac: BdAddr,
    paired_key: &'static [u8],
    pub paired_mac_reverse: BdAddr,  // needed by I/O code for early filtering
}

impl VictronBLE {
    pub fn new() -> Self {
        VictronBLE {
            paired_key: VICTRON_KEY,
            paired_mac_reverse: BdAddr::new(REV_MAC),
            paired_mac: BdAddr::new(REV_MAC),
        }
    }


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
                debug!("Victron Data: {:?}", crate::fmt::Debug2Format(&device_state));
            }
            Err(e) => {
                warn!("Victron Data Error: {:?}", crate::fmt::Debug2Format(&e));
            }
        }
    }
}

impl EventHandler for VictronBLE {
    fn on_vendor(&self, vendor: &Vendor) {
        info!("vendor: {:?}", vendor);
    }

    #[link_section = ".iram1"]
    fn on_adv_reports(&self, mut it: LeAdvReportsIter<'_>) {
        while let Some(Ok(report)) = it.next() {
            if report.addr != self.paired_mac {
                warn!(
                    "ignoring {:?}, that has unexpectedly passed the scan filter",
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