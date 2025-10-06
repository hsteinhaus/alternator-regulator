mod lvgl;

use crate::board::driver::display::DisplayDriver;
use crate::io::PROCESS_DATA;
use crate::ui::lvgl::{Bar, Label, Meter, Widget};
use core::ffi::{c_char, c_void, CStr};
use defmt::{debug, warn};
use embassy_time::{Duration, Timer};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{DrawTarget, Point};
use embedded_graphics::primitives::Rectangle;
use lvgl_rust_sys::{
    lv_align_t, lv_area_t, lv_color_t, lv_disp_drv_t, lv_disp_flush_ready, lv_init, lv_log_register_print_cb,
    lv_obj_set_style_pad_bottom, lv_obj_set_style_pad_left, lv_obj_set_style_pad_right, lv_obj_set_style_pad_top,
    lv_scr_act, lv_text_align_t, lv_tick_inc, lv_timer_handler, LV_ALIGN_RIGHT_MID, LV_DISP_DEF_REFR_PERIOD,
    LV_TEXT_ALIGN_RIGHT,
};
use static_cell::StaticCell;

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

extern "C" {
    pub fn lvgl_disp_init(
        flush_cb: unsafe extern "C" fn(*mut lv_disp_drv_t, *const lv_area_t, *mut lv_color_t),
        user_data: *mut c_void,
    );
}

// void my_disp_flush( lv_disp_drv_t *disp_drv, const lv_area_t *area, lv_color_t *color_p )
unsafe extern "C" fn flush_cb(disp_drv_p: *mut lv_disp_drv_t, area_p: *const lv_area_t, color_p: *mut lv_color_t) {
    if let Some(disp_drv) = disp_drv_p.as_mut() {
        if let Some(area) = area_p.as_ref() {
            let p1 = Point::new(area.x1 as i32, area.y1 as i32);
            let p2 = Point::new(area.x2 as i32, area.y2 as i32);
            let r = Rectangle::with_corners(p1, p2);
            let w = r.size.width;
            let h = r.size.height;
            debug!("Flushing {}..{}", p1, p2);
            if let Some(display) = (disp_drv.user_data as *mut DisplayDriver).as_mut() {
                if !color_p.is_null() {
                    let color_p = color_p as *const Rgb565;
                    let colors_it = core::slice::from_raw_parts(color_p, (w * h) as usize).iter().cloned();
                    display
                        .fill_contiguous(&r, colors_it)
                        .expect("Failed to fill contiguous color");
                } else {
                    warn!("color_p is null");
                }
            } else {
                warn!("user_data is null");
            }
        } else {
            warn!("area_p is null");
        }
    //ip67 kurzes Ventil
    } else {
        warn!("disp_drv_p is null");
    }
    lv_disp_flush_ready(disp_drv_p);
    debug!("Flush done");
}

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
        lvgl_disp_init(flush_cb, &display_driver as *const DisplayDriver as *mut c_void);

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
