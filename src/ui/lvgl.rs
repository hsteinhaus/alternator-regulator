use alloc::ffi::CString;
use core::ffi::c_char;
use lvgl_rust_sys::*;

pub struct Meter {
    handle: *mut lv_obj_t,
    needle: *mut lv_meter_indicator_t,
}

impl Meter {
    pub fn new(parent: *mut lv_obj_t) -> Self {
        unsafe {
            let meter = lv_meter_create(parent);
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

            Meter {
                handle: meter,
                needle,
            }
        }
    }

    pub fn set_value(&self, value: i32) {
        unsafe {
            lv_meter_set_indicator_value(self.handle, self.needle, value);
        }
    }

    pub fn get_handle(&self) -> *mut lv_obj_t {
        self.handle
    }
}

pub struct Label {
    handle: *mut lv_obj_t,
}

impl Label {
    /// Creates a new `Label` on the specified parent widget.
    pub fn new(parent: *mut lv_obj_t) -> Self {
        let handle = unsafe { lv_label_create(parent) };
        Label { handle }
    }

    pub fn x(self, x: i32) -> Self {
        unsafe { lv_obj_set_x(self.handle, x as lv_coord_t) };
        self
    }

    pub fn y(self, y: i32) -> Self {
        unsafe { lv_obj_set_x(self.handle, y as lv_coord_t) };
        self
    }

    pub fn width(self, width: i32) -> Self {
        unsafe { lv_obj_set_width(self.handle, width as lv_coord_t) };
        self
    }

    pub fn height(self, height: i32) -> Self {
        unsafe { lv_obj_set_height(self.handle, height as lv_coord_t) };
        self
    }

    /// Sets the text of the label.
    pub fn text(self, text: &str) -> Self {
        let c_str = CString::new(text).unwrap();
        let c_ptr = c_str.as_ptr() as *mut c_char;
        unsafe { lv_label_set_text(self.handle, c_ptr) };
        self
    }

    /// Aligns the label on the specified parent with the given offsets.
    pub fn align(self, alignment: lv_align_t, x_offset: i32, y_offset: i32) -> Self {
        unsafe { lv_obj_align(self.handle, alignment, x_offset as lv_coord_t, y_offset as lv_coord_t) };
        self
    }

    /// Aligns the label text
    pub fn text_align(self, text_align: lv_text_align_t) -> Self {
        unsafe { lv_obj_set_style_text_align(self.handle, text_align, 0) };
        self
    }

    /// Sets the font of the label text.
    pub fn font(self, font: *const lv_font_t) -> Self {
        unsafe { lv_obj_set_style_text_font(self.handle, font, 0) };
        self
    }

    /// Returns the internal handle of the label.
    pub fn get_handle(&self) -> *mut lv_obj_t {
        self.handle
    }
}

pub struct Bar {
    handle: *mut lv_obj_t,
}

impl Bar {
    pub fn new(parent: *mut lv_obj_t,) -> Self {
        let handle = unsafe{ lv_bar_create(parent) };
        Bar { handle }
    }

    pub fn width(self, width: i32) -> Self {
        unsafe { lv_obj_set_width(self.handle, width as lv_coord_t) };
        self
    }

    pub fn height(self, height: i32) -> Self {
        unsafe { lv_obj_set_height(self.handle, height as lv_coord_t) };
        self
    }

    pub fn range(self, from: i32, to: i32) -> Self {
        unsafe { lv_bar_set_range(self.handle, from, to) };
        self
    }

    pub fn align(self, alignment: lv_align_t, x_offset: i32, y_offset: i32) -> Self {
        unsafe { lv_obj_align(self.handle, alignment, x_offset as lv_coord_t, y_offset as lv_coord_t); };
        self
    }

    pub fn get_handle(&self) -> *mut lv_obj_t {
        self.handle
    }
}
