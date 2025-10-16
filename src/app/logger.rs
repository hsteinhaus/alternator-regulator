use alloc::boxed::Box;
use defmt::Debug2Format;
use embassy_time::{Duration, Ticker};
use embedded_sdmmc::{Error, File, Mode, SdCardError, TimeSource, Timestamp, VolumeIdx, VolumeManager};
use heapless::{format, String};

use crate::app::shared::{PROCESS_DATA, REGULATOR_MODE, RM_LEN, SETPOINT};
use crate::board::startup::SdCardType;


type FileType<'a> = File<'a, SdCardType, EmbassyTimeSource, 4, 4, 1>;
type VolumeManagerType = VolumeManager<SdCardType, EmbassyTimeSource>;

#[derive(Default)]
pub struct EmbassyTimeSource();

struct Logger {
    volume_mgr: VolumeManagerType,
}

impl Logger {
    const FN_LEN: usize = 5+1+3;

    pub async fn new(card: SdCardType) -> Self {
        let mut ticker = Ticker::every(Duration::from_secs(1));
        let size = loop {
            if let Ok(size) = card.num_bytes() {
                break size;
            }
            ticker.next().await;
        };
        info!("SD card size: {:?}", size);
        let volume_mgr = VolumeManager::new(card, EmbassyTimeSource::default());
        Self { volume_mgr }
    }

    pub async fn open(&self) -> Result<Box<FileType<'_>>, Error<SdCardError>> {
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

        let fname: String<{Self::FN_LEN}> = format!("{:05}.CSV", index + 1).unwrap();
        let f = dir.open_file_in_dir(fname.as_str(), Mode::ReadWriteCreateOrAppend)?;
        Ok(Box::new(f))
    }

    pub async fn log<'a>(&self, file: &'a FileType<'a>) -> Result<(), Error<SdCardError>> {
        let now = embassy_time::Instant::now();
        let mut mode: String<RM_LEN> = String::new();
        REGULATOR_MODE.lock(|rm|mode.push_str(rm.borrow().as_str())).unwrap();
        let line: String<800> = format!("{};{};{};{}\n", now.as_millis() as u64, mode, PROCESS_DATA, SETPOINT).unwrap();
        debug!("{}", Debug2Format(&line));
        file.write(line.as_bytes())?;
        file.flush()
    }
}

#[embassy_executor::task]
pub async fn logger(card: SdCardType) -> () {
    let logger = Logger::new(card).await;
    let file = logger.open().await.unwrap();

    let mut ticker = Ticker::every(Duration::from_secs(1));
    //    info!("SD card size: {:?}", logger.wait_for_card_and_get_size().await);
    loop {
        logger.log(&file).await.expect("Failed to log");
        ticker.next().await;
    }
}

impl TimeSource for EmbassyTimeSource {
    // In theory you could use the RTC of the rp2040 here, if you had
    // any external time synchronizing device.
    fn get_timestamp(&self) -> Timestamp {
        let now = embassy_time::Instant::now();
        let secs = now.as_secs();

        // Convert seconds since boot to a basic timestamp
        // Note: This is a simplified conversion, you might want to add real-time clock support
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
