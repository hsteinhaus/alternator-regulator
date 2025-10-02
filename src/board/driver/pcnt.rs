use core::cell::RefCell;
use core::fmt::Display;
use core::sync::atomic::{AtomicI32, Ordering};
use embassy_time::{Duration, Timer};
use esp_hal::{
    gpio::Input,
    pcnt::{channel, unit, Pcnt},
    peripherals::PCNT,
    xtensa_lx::_export::critical_section,
};

static UNIT0: critical_section::Mutex<RefCell<Option<unit::Unit<'static, 1>>>> =
    critical_section::Mutex::new(RefCell::new(None));

pub static RPM: AtomicI32 = AtomicI32::new(0);

pub struct PcntDriver {}

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

        critical_section::with(|cs| UNIT0.borrow_ref_mut(cs).replace(u0));
        Ok(Self {})
    }
}

#[embassy_executor::task]
pub async fn rpm_task() -> ! {
    const POLE_PAIRS: f32 = 6.;
    const LOOP_DELAY_MS: u64 = 100;
    const PULLEY_RATIO: f32 = 53.7 / 128.2;

    loop {
        let mut c = 0;
        critical_section::with(|cs| {
            let mut u0 = UNIT0.borrow_ref_mut(cs);
            if let Some(u0) = u0.as_mut() {
                c = u0.counter.get();
                u0.clear();
            }
        });

        let rpm = c as f32
            * 60.                            // Hz -> rpm
            * (1./POLE_PAIRS/2.)            // 6 pole pairs, 2 imp per rev
            * (1000./LOOP_DELAY_MS as f32)   // interval
            * PULLEY_RATIO; // belt ratio
        RPM.store(rpm as i32, Ordering::SeqCst);
        // info!("RPM: {}", rpm);
        Timer::after(Duration::from_millis(LOOP_DELAY_MS)).await;
    }
}
