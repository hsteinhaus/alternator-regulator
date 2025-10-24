use esp_hal::gpio::{AnyPin, Level, Output, OutputConfig};

pub struct LedResources<'a> {
    pub core0: AnyPin<'a>,
    pub core1: AnyPin<'a>,
    pub user: AnyPin<'a>,
}

impl LedResources<'static> {
    pub fn into_leds(self) -> Leds<'static> {
        Leds {
            core0: Output::new(self.core0, Level::Low, OutputConfig::default()),
            core1: Output::new(self.core1, Level::Low, OutputConfig::default()),
            user: Output::new(self.user, Level::Low, OutputConfig::default()),
        }
    }
}

#[allow(dead_code)]
pub struct Leds<'a> {
    pub core0: Output<'a>,
    pub core1: Output<'a>,
    pub user: Output<'a>,
}
