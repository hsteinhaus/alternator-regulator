use embassy_time::{Duration, Timer};
use esp_hal::analog::adc::{Adc, AdcPin, AdcConfig, Attenuation, AdcChannel, RegisterAccess};
use esp_hal::{Blocking, peripherals};
use esp_hal::gpio::AnalogPin;

pub type AdcDriverType = AdcDriver<'static, peripherals::ADC2<'static>, peripherals::GPIO26<'static>>;

pub struct AdcDriver<'a, A, G> {
    pub adc: Adc<'a, A, Blocking >,
    pub pin: AdcPin<G, A>,
}

impl<'a, A, G> AdcDriver<'a, A, G>
where
    A: RegisterAccess + 'a,
    G: AdcChannel + AnalogPin + 'a
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
                defmt::info!("instant return from adc: {}", r);
                return r;
            }
            defmt::info!("wait for adc");
            Timer::after(Duration::from_millis(100)).await;
        }

    }

}


#[embassy_executor::task]
pub async fn adc_task(mut adc: AdcDriverType) -> ! {
    loop {
        let r = adc.read_oneshot().await;
        defmt::info!("adc: {}", r);
        Timer::after(Duration::from_millis(4900)).await;
    }
}
