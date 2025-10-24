use alloc::boxed::Box;
use embassy_time::{Duration, Ticker};
use embedded_sdmmc::{Error, File, Mode, SdCardError, TimeSource, Timestamp, VolumeIdx, VolumeManager};
use heapless::{format, String};
use thiserror_no_std::Error;

use crate::app::shared::{PROCESS_DATA, REGULATOR_MODE, RM_LEN, SETPOINT};
use crate::board::io::spi2::SdCardType;
use crate::fmt::Debug2Format;

type FileType<'a> = File<'a, SdCardType, EmbassyTimeSource, 4, 4, 1>;
type VolumeManagerType = VolumeManager<SdCardType, EmbassyTimeSource>;

#[derive(Default)]
pub struct EmbassyTimeSource();

#[derive(Debug, Error)]
pub enum LoggerError {
    #[error("SD card error")]
    SdCard(#[from] Error<SdCardError>),

    #[error("Heapless string to small for format: {0}")]
    Heapless(#[from] heapless::CapacityError),

    #[error("Format error: {0}")]
    Format(#[from] core::fmt::Error),

    #[error("SD card not present")]
    NoCardError(#[from] SdCardError),
}

struct DataLogger {
    volume_mgr: VolumeManagerType,
}

pub const LINE_LEN: usize = 800;

impl DataLogger {
    const FN_LEN: usize = 5 + 1 + 3;

    pub async fn new(card: SdCardType) -> Result<Self, LoggerError> {
        let size = card.num_bytes()?;
        info!("SD card size: {:?}", size);
        let volume_mgr = VolumeManager::new(card, EmbassyTimeSource::default());
        Ok(Self { volume_mgr })
    }

    pub async fn open(&self) -> Result<Box<FileType<'_>>, LoggerError> {
        let volume0 = self.volume_mgr.open_volume(VolumeIdx(0))?;
        let volume0 = Box::leak(Box::new(volume0));
        debug!("Volume 0: {:?}", Debug2Format(&volume0));

        let mut dir = volume0.open_root_dir()?;
        dir.make_dir_in_dir("logs").ok(); // create the directory if it doesn't exist
        dir.change_dir("logs")?;
        let dir = Box::leak(Box::new(dir));

        let mut index = 0;
        dir.iterate_dir(|f| {
            let fname = f.name.base_name();
            let num_or_not = u32::from_ascii(fname);
            if let Ok(num) = num_or_not {
                debug!("Found file: {:?}", Debug2Format(&f.name));
                index = num.max(index);
            }
        })?;

        let fname: String<{ Self::FN_LEN }> = format!("{:05}.CSV", index + 1)?;
        let file = dir.open_file_in_dir(fname.as_str(), Mode::ReadWriteCreateOrAppend)?;

        let line =
            format!({ LINE_LEN }; "{};{};{};;{}\n", "Timestamp", "Mode", PROCESS_DATA.get_meta(), SETPOINT.get_meta())?;
        debug!("{:?}", Debug2Format(&line));
        file.write(line.as_bytes())?;
        file.flush()?;

        Ok(Box::new(file))
    }

    pub async fn log<'a>(&self, file: &'a FileType<'a>) -> Result<(), LoggerError> {
        let now = embassy_time::Instant::now();
        let mut mode: String<RM_LEN> = String::new();
        REGULATOR_MODE.lock(|rm| mode.push_str(rm.borrow().as_str()))?;
        let line = format!({ LINE_LEN }; "{};{};{};;{}\n", now.as_millis() as u64, mode, PROCESS_DATA, SETPOINT)?;
        debug!("{:?}", Debug2Format(&line));
        file.write(line.as_bytes())?;
        file.flush()?;
        Ok(())
    }
}

#[embassy_executor::task]
pub async fn logger(card: SdCardType) -> () {
    let Ok(logger) = DataLogger::new(card).await else {
        warn!("Could not init SD card, disabling CSV logger");
        return;
    };
    let Ok(file) = logger.open().await else {
        warn!("Could not log file, disabling CSV logger");
        return;
    };

    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        logger.log(&file).await.unwrap_or_else(|err| {
            error!("Could not log to SD card: {:?}", Debug2Format(&err));
        });
        ticker.next().await;
    }
}

impl TimeSource for EmbassyTimeSource {
    fn get_timestamp(&self) -> Timestamp {
        let now = embassy_time::Instant::now();
        let secs = now.as_secs();
        Timestamp {
            year_since_1970: (secs / (365 * 24 * 3600)) as u8,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: ((secs % (24 * 3600)) / 3600) as u8,
            minutes: ((secs % 3600) / 60) as u8,
            seconds: (secs % 60) as u8,
        }
    }
}

pub trait LoggerMeta {
    fn get_meta(&self) -> String<LINE_LEN>;
}
