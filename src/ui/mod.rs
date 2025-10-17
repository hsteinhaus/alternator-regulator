use core::ffi::{c_char, c_void, CStr};
use embassy_time::{Duration, Instant, Timer};
use heapless::String;
use lvgl_rust_sys::{
    lv_align_t, lv_disp_get_default, lv_init, lv_log_register_print_cb, lv_obj_set_style_pad_bottom,
    lv_obj_set_style_pad_left, lv_obj_set_style_pad_right, lv_obj_set_style_pad_top, lv_scr_act, lv_text_align_t,
    lv_timer_handler, LV_ALIGN_RIGHT_MID, LV_TEXT_ALIGN_RIGHT,
};

use self::lvgl::{Bar, Label, Meter, Widget};
use self::lvgl_buffers::lvgl_disp_init;
use crate::app::shared::{MAX_FIELD_CURRENT, MAX_FIELD_VOLTAGE, PROCESS_DATA, REGULATOR_MODE, RM_LEN};
use crate::board::driver::display::DisplayDriver;
use crate::ui::lvgl::WidgetError;

mod lvgl;
mod lvgl_buffers;

#[allow(unused)]
#[derive(Debug, Default)]
struct Widgets<'a> {
    meter: Meter<'a>,
    field_voltage_bar: Bar,
    field_current_bar: Bar,
    field_voltage_label: Label<'a>,
    field_current_label: Label<'a>,
}

#[no_mangle]
unsafe extern "C" fn lvgl_log_print(c_str: *const c_char) {
    let text = unsafe { CStr::from_ptr(c_str) };
    warn!("LVGL: {}", text.to_str().unwrap());
}

#[no_mangle]
#[link_section = ".iram1"]
pub extern "C" fn get_tick_ms() -> u32 {
    let ms = Instant::now().as_millis() as u32;
    ms
}

impl<'a> Widgets<'a> {
    fn create() -> Result<Self, WidgetError> {
        unsafe {
            lv_obj_set_style_pad_top(lv_scr_act(), 6, 0);
            lv_obj_set_style_pad_bottom(lv_scr_act(), 6, 0);
            lv_obj_set_style_pad_left(lv_scr_act(), 12, 0);
            lv_obj_set_style_pad_right(lv_scr_act(), 12, 0);
        };
        let screen = unsafe { lv_scr_act() };
        assert!(!screen.is_null());

        // Create and configure the meter
        let mut meter = Meter::new(screen)?;
        meter.set_value(0.)?;

        // Create bars for field voltage and current
        let field_voltage_bar = Bar::new(screen)?.width(12).height(228).range(0., MAX_FIELD_VOLTAGE);

        let field_current_bar =
            Bar::new(screen)?
                .width(12)
                .height(228)
                .range(0., MAX_FIELD_CURRENT)
                .align(LV_ALIGN_RIGHT_MID as lv_align_t, 0, 0);

        // Create labels for field voltage and current
        let field_voltage_label = Label::new(screen, "V")?;
        field_voltage_label.x(18).text("1.3V")?;

        let field_current_label = Label::new(screen, "A")?;
        field_current_label
            .x(228)
            .width(50)
            .text("-0.0A")?
            .text_align(LV_TEXT_ALIGN_RIGHT as lv_text_align_t);

        Ok(Widgets {
            meter,
            field_voltage_bar,
            field_current_bar,
            field_voltage_label,
            field_current_label,
        })
    }

    pub fn update(&mut self) -> Result<(), lvgl::WidgetError> {
        let current = PROCESS_DATA.bat_current.load(core::sync::atomic::Ordering::Relaxed);
        let field_voltage = PROCESS_DATA.field_voltage.load(core::sync::atomic::Ordering::Relaxed);
        let field_current = PROCESS_DATA.field_current.load(core::sync::atomic::Ordering::Relaxed);
        let rpm = PROCESS_DATA.rpm.load(core::sync::atomic::Ordering::Relaxed);

        if current.is_finite() {
            self.meter.set_value(current)?;
        }
        if field_voltage.is_finite() {
            self.field_voltage_bar.set_value(field_voltage)?;
            self.field_voltage_label.set_value(field_voltage)?;
        }
        if field_current.is_finite() {
            self.field_current_bar.set_value(field_current)?;
            self.field_current_label.set_value(field_current)?;
        }
        if rpm.is_finite() {
            self.meter.set_rpm(rpm)?;
        }

        REGULATOR_MODE.lock(|rm| {
            let rm: &String<RM_LEN> = &rm.borrow();
            self.meter.set_state(rm).ok();
        });

        Ok(())
    }
}

// async fn lvgl_refresh_task(disp_refr: *mut lv_disp_t) {
//     // this async fn replaces LVGL's central refresh routine `_lv_disp_refr_timer()`;
//     unsafe {
//         let disp_refr = disp_refr.as_mut().unwrap();
//         lv_obj_update_layout(disp_refr.act_scr);
//         yield_now().await;
//         lv_obj_update_layout(disp_refr.top_layer);
//         yield_now().await;
//         lv_obj_update_layout(disp_refr.sys_layer);
//         yield_now().await;
//
//         lv_refr_join_area();
//         yield_now().await;
//         refr_sync_areas();
//         yield_now().await;
//         refr_invalid_areas();
//     }
//
// }

#[embassy_executor::task]
pub async fn ui_task(mut display_driver: DisplayDriver) {
    unsafe {
        // initialize LVGL
        lv_init();
        lv_log_register_print_cb(Some(lvgl_log_print)); /* register print function for debugging */
        lvgl_disp_init(&display_driver as *const DisplayDriver as *mut c_void);

        // Create the widgets
        let Ok(mut widgets) = Widgets::create() else
        {
            warn!("Could not create LVGL widgets, disabling UI");
            return;
        };

        // UI loop
        lv_timer_handler(); // first rendering takes a long time, so do it once befor turing on the backlight
        display_driver.bl_on();
        let _disp = lv_disp_get_default();
        loop {
            widgets
                .update()
                .unwrap_or_else(|e| warn!("Failed to update widgets: {:?}", e));
            lv_timer_handler();
            //            lv_refr_now(disp);
            //            lvgl_refresh_task(disp).await;
            Timer::after(Duration::from_millis(100)).await;
        }
    }
}
