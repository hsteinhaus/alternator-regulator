use core::fmt::Display;
use esp_hal::{
    gpio::Input,
    pcnt::{channel, Pcnt, unit},
    peripherals::PCNT,
};


pub struct PcntDriver {
    pub pcnt_unit: unit::Unit<'static, 1>
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
    pub fn initialize(pcnt: PCNT<'static>, rpm_pin: Input<'static>) -> Result<Self, Error> {
        // Initialize Pulse Counter (PCNT) unit with limits and filter settings
        let pcnt = Pcnt::new(pcnt);
        //        pcnt.set_interrupt_handler(interrupt_handler);
        let u0 = pcnt.unit1;
        //        u0.set_low_limit(Some(-100)).expect("set low limit failed");
        //        u0.set_high_limit(Some(100)).expect("set high limit failed");
        //        u0.set_filter(Some(min(10u16 * 80, 1023u16))).expect("set filter failed");
        u0.clear();

        // Set up channels with control and edge signals
        let ch0 = &u0.channel0;
        ch0.set_edge_signal(rpm_pin);
        ch0.set_input_mode(channel::EdgeMode::Increment, channel::EdgeMode::Increment);

        // Enable interrupts and resume pulse counter unit
        u0.listen();
        u0.resume();
        Ok(Self {pcnt_unit: u0})
    }
}

