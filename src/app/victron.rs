use bt_hci::event::Vendor;
use bt_hci::param::{BdAddr, LeAdvReportsIter, LeExtAdvReportsIter};
use trouble_host::advertise::AdStructure;
use trouble_host::prelude::EventHandler;
use victron_ble::DeviceState;

use crate::app::shared::PROCESS_DATA;

struct VictronDevice {
    pub mac: [u8; 6],
    pub key: [u8; 16],
}

impl VictronDevice {
    pub fn bd_addr(&self) -> BdAddr {
        let mut reversed = [0u8; 6];
        reversed.copy_from_slice(&self.mac);
        reversed.reverse();
        BdAddr::new(reversed)
    }
}

pub struct VictronBLE {
    paired_key: &'static [u8],
    pub paired_mac: BdAddr,
}

impl VictronBLE {
    const VICTRON_ID: u16 = 0x02e1;
    const VICTRON_DEVICES: [VictronDevice; 3] = [
        VictronDevice {
            // AC Charger - just for SW testing
            mac: [0xc0, 0x12, 0x9b, 0x97, 0x7f, 0xb8],
            key: [
                0x34, 0xa4, 0x20, 0xf8, 0x6f, 0xa0, 0x37, 0x50, 0x8a, 0x83, 0x47, 0xf6, 0x21, 0x4d, 0xc1, 0xf4,
            ],
        },
        VictronDevice {
            // SmartShunt 300A (alternator)
            mac: [0xf9, 0x3c, 0xeb, 0x5e, 0xf4, 0x75],
            key: [
                0xe8, 0xe4, 0xd8, 0x14, 0x4a, 0x72, 0x49, 0x2e, 0x8e, 0x8b, 0x2b, 0x9c, 0x93, 0x78, 0xbd, 0xfb,
            ],
        },
        VictronDevice {
            // SmartShunt 500A (battery)
            mac: [0xd9, 0xd5, 0x51, 0x59, 0x70, 0x4d],
            key: [
                0x13, 0xc6, 0xbf, 0xf8, 0xdb, 0xef, 0xcf, 0x2d, 0xd5, 0xd5, 0x07, 0x79, 0x8d, 0xc1, 0x0f, 0x9e,
            ],
        },
    ];

    pub fn new() -> Self {
        VictronBLE {
            paired_key: &VictronBLE::VICTRON_DEVICES[0].key,
            paired_mac: VictronBLE::VICTRON_DEVICES[0].bd_addr(),
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
                                if company_identifier == VictronBLE::VICTRON_ID {
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
