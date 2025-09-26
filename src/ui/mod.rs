mod lvgl;

use crate::board::driver::display::DisplayDriver;
use alloc::ffi::CString;
use core::ffi::{c_char, c_void, CStr};
use defmt::{debug, warn};
use embassy_time::{Duration, Timer};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{DrawTarget, Point};
use embedded_graphics::primitives::Rectangle;
use lvgl_rust_sys::{lv_area_t, lv_bar_create, lv_bar_set_range, lv_color_hex, lv_color_t, lv_disp_drv_t, lv_disp_flush_ready, lv_font_montserrat_14, lv_font_montserrat_40, lv_init, lv_label_create, lv_label_set_text, lv_log_register_print_cb, lv_meter_add_arc, lv_meter_add_needle_line, lv_meter_add_scale, lv_meter_create, lv_meter_set_indicator_value, lv_obj_align, lv_obj_set_height, lv_obj_set_style_bg_color, lv_obj_set_style_pad_bottom, lv_obj_set_style_pad_left, lv_obj_set_style_pad_right, lv_obj_set_style_pad_top, lv_obj_set_style_text_align, lv_obj_set_style_text_font, lv_obj_set_width, lv_obj_set_x, lv_obj_set_y, lv_scr_act, lv_text_align_t, lv_tick_inc, lv_timer_handler, LV_ALIGN_CENTER, LV_ALIGN_LEFT_MID, LV_ALIGN_RIGHT_MID, LV_DISP_DEF_REFR_PERIOD, LV_PART_MAIN, LV_TEXT_ALIGN_RIGHT};

extern "C" {
    pub fn lvgl_disp_init(
        flush_cb: unsafe extern "C" fn(*mut lv_disp_drv_t, *const lv_area_t, *mut lv_color_t),
        user_data: *mut c_void,
    );
}

// void my_disp_flush( lv_disp_drv_t *disp_drv, const lv_area_t *area, lv_color_t *color_p )
unsafe extern "C" fn flush_cb(
    disp_drv_p: *mut lv_disp_drv_t,
    area_p: *const lv_area_t,
    color_p: *mut lv_color_t,
) {
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
                    let colors_it = core::slice::from_raw_parts(color_p, (w * h) as usize)
                        .iter()
                        .cloned();
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
}

unsafe extern "C" fn my_print(c_str: *const c_char) {
    let text = unsafe { CStr::from_ptr(c_str) };
    warn!("LVGL: {}", text.to_str().unwrap());
}

#[embassy_executor::task]
pub async unsafe fn ui_task(display_driver: DisplayDriver) {
    lv_init();
    lv_log_register_print_cb(Some(my_print)); /* register print function for debugging */

    lvgl_disp_init(
        flush_cb,
        &display_driver as *const DisplayDriver as *mut c_void,
    );
    //lv_obj_set_style_bg_color(lv_scr_act(), lv_color_hex(0x003a57), LV_PART_MAIN);
    lv_obj_set_style_pad_top(lv_scr_act(), 6, 0);
    lv_obj_set_style_pad_bottom(lv_scr_act(), 6, 0);
    lv_obj_set_style_pad_left(lv_scr_act(), 12, 0);
    lv_obj_set_style_pad_right(lv_scr_act(), 12, 0);

    let meter = lv_meter_create(lv_scr_act());
    lv_obj_align(meter, LV_ALIGN_CENTER.try_into().unwrap(), 0, 0);
    lv_obj_set_width(meter, 228);
    lv_obj_set_height(meter, 228);
    let scale = lv_meter_add_scale(meter);
    (*scale).min = 0;
    (*scale).max = 150;
    (*scale).angle_range = 240;
    (*scale).rotation = 150;
    let needle = lv_meter_add_needle_line(meter, scale, 5, lv_color_hex(0xff0000), -4);
    let green_arc = lv_meter_add_arc(meter, scale, 10, lv_color_hex(0x009f00), 10);
    (*green_arc).start_value = 0;
    (*green_arc).end_value = 99;
    let yellow_arc = lv_meter_add_arc(meter, scale, 10, lv_color_hex(0xffff00), 10);
    (*yellow_arc).start_value = 100;
    (*yellow_arc).end_value = 119;
    let red_arc = lv_meter_add_arc(meter, scale, 10, lv_color_hex(0xff0000), 10);
    (*red_arc).start_value = 120;
    (*red_arc).end_value = 150;
    let scale2 = lv_meter_add_scale(meter);
    (*scale2).min = 0;
    (*scale2).max = 150;
    (*scale2).angle_range = 240;
    (*scale2).rotation = 150;
    (*scale2).tick_width = 1;
    (*scale2).tick_cnt = 51;
    (*scale2).tick_length = 10;
    (*scale2).tick_color = lv_color_hex(0x000000);
    (*scale2).tick_major_nth = 5;
    (*scale2).tick_major_width = 2;
    (*scale2).tick_major_length = 10;
    (*scale2).tick_major_color = lv_color_hex(0x404040);
    (*scale2).label_gap = 10;
    lv_obj_set_style_text_font(meter, &lv_font_montserrat_14, 0);
    lv_meter_set_indicator_value(meter, needle, 42);

    let label = lv_label_create(meter);
    lv_obj_align(label, LV_ALIGN_CENTER.try_into().unwrap(), 0, 50);
    let c_str = CString::new("-.-").unwrap();
    let c_world: *mut c_char = c_str.as_ptr() as *mut c_char;
    lv_obj_set_style_text_font(label, &lv_font_montserrat_40, 0);
    lv_label_set_text(label, c_world);

    let label = lv_label_create(meter);
    lv_obj_align(label, LV_ALIGN_CENTER.try_into().unwrap(), 0, 75);
    let c_str = CString::new("Amps").unwrap();
    let c_world: *mut c_char = c_str.as_ptr() as *mut c_char;
    lv_label_set_text(label, c_world);

    let field_voltage_bar = lv_bar_create(lv_scr_act());
    lv_obj_set_width(field_voltage_bar, 12);
    lv_obj_set_height(field_voltage_bar, 228);
    lv_bar_set_range(field_voltage_bar, 0, 300);

    let field_voltage_label = lv_label_create(lv_scr_act());
    lv_obj_set_x(field_voltage_label, 18);
    let c_str = CString::new("1.3V").unwrap();
    let c_world: *mut c_char = c_str.as_ptr() as *mut c_char;
    lv_label_set_text(field_voltage_label, c_world);

    let field_current_bar = lv_bar_create(lv_scr_act());
    lv_obj_set_width(field_current_bar, 12);
    lv_obj_set_height(field_current_bar, 228);
    lv_bar_set_range(field_current_bar, 0, 50);
    lv_obj_align(field_current_bar, LV_ALIGN_RIGHT_MID.try_into().unwrap(), 0, 0);

    let field_current_label = lv_label_create(lv_scr_act());
    lv_obj_set_x(field_current_label, 226);
    lv_obj_set_width(field_current_label, 50);
    lv_obj_set_style_text_align(field_current_label, LV_TEXT_ALIGN_RIGHT as lv_text_align_t, 0);
    let c_str = CString::new("-0.0A").unwrap();
    let c_world: *mut c_char = c_str.as_ptr() as *mut c_char;
    lv_label_set_text(field_current_label, c_world);

    loop {
        lv_timer_handler();
        Timer::after(Duration::from_millis(LV_DISP_DEF_REFR_PERIOD as u64)).await;
        lv_tick_inc(LV_DISP_DEF_REFR_PERIOD);
    }
}
