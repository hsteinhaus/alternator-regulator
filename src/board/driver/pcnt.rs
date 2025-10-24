use core::fmt::Display;
use esp_hal::{
    gpio::Input,
    pcnt::{channel, unit, Pcnt},
    peripherals::PCNT,
};

pub struct PcntDriver {
    pub pcnt_unit: unit::Unit<'static, 1>,
}

impl PcntDriver {
    pub fn get_and_reset(&mut self) -> i16 {
        let c = self.pcnt_unit.counter.get();
        self.pcnt_unit.clear();
        c
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    Unknown,
    PcntDriverError,
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Error")
    }
}

impl PcntDriver {
    pub fn new(pcnt: PCNT<'static>, rpm_pin: Input<'static>) -> Result<Self, Error> {
        // Initialize Pulse Counter (PCNT) unit with limits and filter settings
        let pcnt = Pcnt::new(pcnt);
        let u0 = pcnt.unit1;
        u0.clear();

        // Set up channels with control and edge signals
        let ch0 = &u0.channel0;
        ch0.set_edge_signal(rpm_pin);
        ch0.set_input_mode(channel::EdgeMode::Increment, channel::EdgeMode::Increment);

        // Enable interrupts and resume pulse counter unit
        u0.listen();
        u0.resume();
        Ok(Self { pcnt_unit: u0 })
    }
}
