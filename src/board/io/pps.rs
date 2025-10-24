use crate::app::shared::{PpsSetMode, PROCESS_DATA, SETPOINT};
use crate::board::driver::pps::{PpsDriver, PpsError};
use core::sync::atomic::Ordering;
use embassy_time::{with_timeout, Duration, Instant, Ticker};
use esp_hal::gpio::AnyPin;
use esp_hal::i2c::master::{AnyI2c, BusTimeout, Config as I2cConfig, I2c};
use esp_hal::time::Rate;
use num_traits::FromPrimitive;


const PPS_LOOP_TIME_MS: u64 = 500;

pub async fn read_pps(pps: &mut PpsDriver) {
    pps.get_voltage()
        .await
        .and_then(|v| Ok(PROCESS_DATA.field_voltage.store(v, Ordering::Relaxed)))
        .ok();
    pps.get_current()
        .await
        .and_then(|i| Ok(PROCESS_DATA.field_current.store(i, Ordering::Relaxed)))
        .ok();
    pps.get_temperature()
        .await
        .and_then(|t| Ok(PROCESS_DATA.pps_temperature.store(t, Ordering::Relaxed)))
        .ok();
    pps.get_input_voltage()
        .await
        .and_then(|v| Ok(PROCESS_DATA.input_voltage.store(v, Ordering::Relaxed)))
        .ok();
    pps.get_running_mode()
        .await
        .and_then(|m| Ok(PROCESS_DATA.pps_mode.store(m as u8, Ordering::Relaxed)))
        .ok();
}

pub async fn write_pps(pps: &mut PpsDriver) -> Result<(), PpsError> {
    let current_limit = SETPOINT.field_current_limit.swap(f32::NAN, Ordering::Relaxed);
    let voltage_limit = SETPOINT.field_voltage_limit.swap(f32::NAN, Ordering::Relaxed);
    let enabled = PpsSetMode::from_u8(
        SETPOINT
            .pps_enabled
            .swap(PpsSetMode::DontTouch as u8, Ordering::Relaxed),
    )
    .ok_or(PpsError::Unsupported)?;
    debug!(
        "write_pps: cl: {:?} vl: {:?} enabled: {:?}",
        current_limit, voltage_limit, enabled
    );

    if !current_limit.is_nan() {
        pps.set_current(current_limit).await?;
    }
    if !voltage_limit.is_nan() {
        pps.set_voltage(voltage_limit).await?;
    }
    match enabled {
        PpsSetMode::Off => {
            pps.enable(false).await?;
        }
        PpsSetMode::On => {
            pps.enable(true).await?;
        }
        PpsSetMode::DontTouch => (),
    };
    Ok(())
}

#[embassy_executor::task]
pub async fn pps_task(pps_resources: PpsResources<'static>) -> () {
    let mut pps = match pps_resources.into_pps() {
        Ok(pps) => pps,
        Err(err) => {
            error!("critical error - PPS startup failed: {:?}", err);
            return;
        },
    };

    let mut ticker = Ticker::every(Duration::from_millis(PPS_LOOP_TIME_MS));
    loop {
        let loop_start = Instant::now();
        trace!("process_data: {:?}", crate::fmt::Debug2Format(&PROCESS_DATA));
        trace!("setpoint: {:?}", crate::fmt::Debug2Format(&SETPOINT));
        with_timeout(Duration::from_millis(PPS_LOOP_TIME_MS * 3), async {
            write_pps(&mut pps)
                .await
                .unwrap_or_else(|e| warn!("PPS write error: {:?}", e));
            read_pps(&mut pps).await;
        })
        .await
        .unwrap_or_else(|_| {
            error!("timeout in io i2c loop");
            ticker.reset_at(Instant::now() - Duration::from_millis(PPS_LOOP_TIME_MS));
        });
        let loop_time = loop_start.elapsed();
        debug!("io loop time: {:?} ms", loop_time.as_millis());
        ticker.next().await;
    }
}

pub struct PpsResources<'a> {
    pub i2c: AnyI2c<'a>,
    pub scl: AnyPin<'a>,
    pub sda: AnyPin<'a>,
}

impl PpsResources<'static> {
    pub fn into_pps(self) -> Result<PpsDriver, PpsError> {
        let i2c = I2c::new(
            self.i2c,
            I2cConfig::default()
                .with_frequency(Rate::from_khz(400))
                .with_timeout(BusTimeout::BusCycles(20)),
        )?
        .with_sda(self.sda)
        .with_scl(self.scl)
        .into_async();
        let pps = PpsDriver::new(i2c, 0x35)?;
        Ok(pps)
    }
}
