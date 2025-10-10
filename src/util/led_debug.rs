use core::cell::OnceCell;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use esp_hal::gpio::Output;

static LED_DEBUG: Mutex<CriticalSectionRawMutex, OnceCell<LedDebug>> = Mutex::new(OnceCell::new());

#[derive(Debug)]
#[allow(dead_code)]
pub struct LedDebug {
    led: Output<'static>,
}

#[allow(dead_code)]
impl LedDebug {
    pub fn create(led: Output<'static>) {
        unsafe {
            LED_DEBUG.lock_mut(|led_oc| {
                led_oc.set(LedDebug { led }).unwrap();
            });
        }
    }

    pub fn begin() {
        unsafe {
            LED_DEBUG.lock_mut(|led| {
                led.get_mut().unwrap().led.set_high();
            });
        }
    }

    pub fn end() {
        unsafe {
            LED_DEBUG.lock_mut(|led| {
                led.get_mut().unwrap().led.set_low();
            });
        }
    }
}
