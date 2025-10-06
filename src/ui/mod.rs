mod lvgl;
mod lvgl_buffers;

use core::ffi::{c_char, c_void, CStr};
use defmt::warn;
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

use lvgl_rust_sys::{
    lv_align_t, lv_init, lv_log_register_print_cb, lv_obj_set_style_pad_bottom, lv_obj_set_style_pad_left,
    lv_obj_set_style_pad_right, lv_obj_set_style_pad_top, lv_scr_act, lv_text_align_t, lv_tick_inc, lv_timer_handler,
    LV_ALIGN_RIGHT_MID, LV_DISP_DEF_REFR_PERIOD, LV_TEXT_ALIGN_RIGHT,
};

use crate::board::driver::display::DisplayDriver;
use crate::io::PROCESS_DATA;
use crate::ui::lvgl::{Bar, Label, Meter, Widget};
use crate::ui::lvgl_buffers::lvgl_disp_init;

#[allow(unused)]
#[derive(Debug, Default)]
struct Widgets {
    meter: Meter,
    field_voltage_bar: Bar,
    field_current_bar: Bar,
    field_voltage_label: Label,
    field_current_label: Label,
}

static WIDGETS: StaticCell<Widgets> = StaticCell::new();

unsafe extern "C" fn lvgl_log_print(c_str: *const c_char) {
    let text = unsafe { CStr::from_ptr(c_str) };
    warn!("LVGL: {}", text.to_str().unwrap());
}

impl Widgets {
    fn create() -> Self {
        unsafe {
            lv_obj_set_style_pad_top(lv_scr_act(), 6, 0);
            lv_obj_set_style_pad_bottom(lv_scr_act(), 6, 0);
            lv_obj_set_style_pad_left(lv_scr_act(), 12, 0);
            lv_obj_set_style_pad_right(lv_scr_act(), 12, 0);
        };
        let screen = unsafe { lv_scr_act() };
        assert!(!screen.is_null());

        // Create and configure the meter
        let mut meter = Meter::new(screen);
        meter.set_value(0.).expect("Failed to set meter value");

        // Create bars for field voltage and current
        let field_voltage_bar = Bar::new(screen).width(12).height(228).range(0., 30.);

        let field_current_bar =
            Bar::new(screen)
                .width(12)
                .height(228)
                .range(0., 5.)
                .align(LV_ALIGN_RIGHT_MID as lv_align_t, 0, 0);

        // Create labels for field voltage and current
        let field_voltage_label = Label::new(screen);
        field_voltage_label.x(18).text("1.3V");

        let field_current_label = Label::new(screen);
        field_current_label
            .x(228)
            .width(50)
            .text("-0.0A")
            .text_align(LV_TEXT_ALIGN_RIGHT as lv_text_align_t);

        Widgets {
            meter,
            field_voltage_bar,
            field_current_bar,
            field_voltage_label,
            field_current_label,
        }
    }

    pub fn update(&mut self) -> Result<(), lvgl::Error> {
        let current = PROCESS_DATA.bat_current.load(core::sync::atomic::Ordering::Relaxed);
        let field_voltage = PROCESS_DATA.field_voltage.load(core::sync::atomic::Ordering::Relaxed);
        let field_current = PROCESS_DATA.field_current.load(core::sync::atomic::Ordering::Relaxed);

        self.meter.set_value(current)?;
        self.field_voltage_bar.set_value(field_voltage)?;
        self.field_voltage_label.set_value(field_voltage)?;
        self.field_current_bar.set_value(field_current)?;
        self.field_current_label.set_value(field_current)?;
        Ok(())
    }
}

#[embassy_executor::task]
pub async fn ui_task(mut display_driver: DisplayDriver) -> ! {
    unsafe {
        // initialize LVGL
        lv_init();
        lv_log_register_print_cb(Some(lvgl_log_print)); /* register print function for debugging */
        lvgl_disp_init(&display_driver as *const DisplayDriver as *mut c_void);

        // Create the widgets
        let widgets = Widgets::create();
        let widgets = WIDGETS.init(widgets);

        // UI loop
        lv_timer_handler(); // first rendering takes a long time, so do it once befor turing on the backlight
        display_driver.bl_on();
        loop {
            widgets
                .update()
                .unwrap_or_else(|e| warn!("Failed to update widgets: {:?}", e));
            lv_tick_inc(LV_DISP_DEF_REFR_PERIOD);
            lv_timer_handler();
            Timer::after(Duration::from_millis(LV_DISP_DEF_REFR_PERIOD as u64)).await;
        }
    }
}
