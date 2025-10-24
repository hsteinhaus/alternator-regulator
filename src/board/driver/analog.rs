use embassy_time::{Duration, Timer};
use esp_hal::{
    analog::adc::{Adc, AdcChannel, AdcConfig, AdcPin, Attenuation, RegisterAccess},
    gpio::AnalogPin,
    peripherals, Blocking,
};


pub type AdcDriverType = AdcDriver<peripherals::ADC2<'static>, peripherals::GPIO26<'static>>;

pub struct AdcDriver<A, G> {
    pub adc: Adc<'static, A, Blocking>,
    pub pin: AdcPin<G, A>,
}

#[allow(unused)]
impl<A, G> AdcDriver<A, G>
where
    A: RegisterAccess + 'static,
    G: AdcChannel + AnalogPin + 'static,
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
