use embassy_time::{Duration, Ticker};
use crate::board::startup::SdCardType;

#[embassy_executor::task]
pub async fn logger(card: SdCardType) -> () {
    info!("Starting pro_main");

    let mut ticker = Ticker::every(Duration::from_secs(1));
    let size = loop {
        ticker.next().await;
        if let Ok(size) = card.num_bytes() {
            break size;
        }
    };
    info!("SD card size: {:?}", size);
    loop {
        ticker.next().await;
    }
}
