use embassy_time::{Duration, Timer};
use esp_hal::analog::adc::{Adc, AdcChannel, AdcConfig, AdcPin, Attenuation, RegisterAccess};
use esp_hal::gpio::AnalogPin;
use esp_hal::{peripherals, Blocking};

pub type AdcDriverType = AdcDriver<'static, peripherals::ADC2<'static>, peripherals::GPIO26<'static>>;

pub struct AdcDriver<'a, A, G> {
    pub adc: Adc<'a, A, Blocking>,
    pub pin: AdcPin<G, A>,
}

impl<'a, A, G> AdcDriver<'a, A, G>
where
    A: RegisterAccess + 'a,
    G: AdcChannel + AnalogPin + 'a,
{
    pub fn initialize(adc1_peripheral: A, analog_pin: G) -> Self {
        let mut adc1_config = AdcConfig::new();
        let pin = adc1_config.enable_pin(analog_pin, Attenuation::_0dB);
        let adc = Adc::new(adc1_peripheral, adc1_config);

        Self { adc, pin }
    }

    pub async fn read_oneshot(&mut self) -> u16 {
        loop {
            let nbr: nb::Result<u16, ()> = self.adc.read_oneshot(&mut self.pin);
            Timer::after(Duration::from_millis(100)).await;
            if let Ok(r) = nbr {
                info!("instant return from adc: {}", r);
                return r;
            }
            info!("wait for adc");
            Timer::after(Duration::from_millis(100)).await;
        }
    }
}
