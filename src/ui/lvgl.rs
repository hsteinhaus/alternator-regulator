// Unfortunately, the official lvgl-rs crate is a big mess ATM.
// This unsafe and probably also unsound hack is at least working...

use core::ffi::c_char;
use defmt::Format;
use heapless::{format, String, CString};
use lvgl_rust_sys::*;
use thiserror_no_std::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unable to format value")]
    FormatError(#[from]core::fmt::Error),
}

impl Format for Error {
    fn format(&self, f: defmt::Formatter) {
        match self {
            Error::FormatError(_) => defmt::write!(f, "FormatError"),
        }
    }
}


#[allow(unused)]
pub trait Widget {
    fn get_handle(&self) -> *mut lv_obj_t;

    /// Aligns the label on the specified parent with the given offsets.
    fn align(&self, alignment: lv_align_t, x_offset: i32, y_offset: i32) -> &Self {
        unsafe {
            lv_obj_align(
                self.get_handle(),
                alignment,
                x_offset as lv_coord_t,
                y_offset as lv_coord_t,
            )
        };
        self
    }
    fn x(&self, x: i32) -> &Self {
        unsafe { lv_obj_set_x(self.get_handle(), x as lv_coord_t) };
        self
    }

    fn y(&self, y: i32) -> &Self {
        unsafe { lv_obj_set_x(self.get_handle(), y as lv_coord_t) };
        self
    }

    fn width(&self, width: i32) -> &Self {
        unsafe { lv_obj_set_width(self.get_handle(), width as lv_coord_t) };
        self
    }

    fn height(&self, height: i32) -> &Self {
        unsafe { lv_obj_set_height(self.get_handle(), height as lv_coord_t) };
        self
    }

    fn set_value(&mut self, value: f32) -> Result<(), Error>;
}

#[derive(Debug, Default)]
pub struct Meter {
    handle: *mut lv_obj_t,
    needle: *mut lv_meter_indicator_t,
    current_label: Label,
}

impl Widget for Meter {
    fn get_handle(&self) -> *mut lv_obj_t {
        self.handle
    }

    fn set_value(&mut self, value: f32) -> Result<(), Error>
    {
        unsafe {
            lv_meter_set_indicator_value(self.handle, self.needle, value as i32);
        }
        let s: String<10> = format!("{:.1}", value)?;
        self.current_label.text(s.as_str());
        Ok(())
    }
}

impl Meter {
    pub fn new(parent: *mut lv_obj_t) -> Self {
        unsafe {
            let meter = lv_meter_create(parent);
            lv_obj_align(meter, LV_ALIGN_CENTER.try_into().unwrap(), 0, 0);
            lv_obj_set_width(meter, 228);
            lv_obj_set_height(meter, 228);

            let scale = lv_meter_add_scale(meter).as_mut().unwrap();
            scale.min = 0;
            scale.max = 150;
            scale.angle_range = 240;
            scale.rotation = 150;

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

            // Create labels for the meter
            let current_label = Label::new(meter);
            current_label
                .text("-.-")
                .align(LV_ALIGN_CENTER as lv_align_t, 0, 50)
                .font(&lv_font_montserrat_40);

            let _static_label = Label::new(meter)
                .text("Amps")
                .align(LV_ALIGN_CENTER as lv_align_t, 0, 75)
                .font(&lv_font_montserrat_14);

            Meter {
                handle: meter,
                current_label,
                needle,
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct Label {
    handle: *mut lv_obj_t,
}

impl Widget for Label {
    fn get_handle(&self) -> *mut lv_obj_t {
        self.handle
    }

    fn set_value(&mut self, value: f32) -> Result<(), Error> {
        let s: String<10> = format!("{:.1}", value)?;
        self.text(s.as_str());
        Ok(())
    }
}

impl Label {
    /// Creates a new `Label` on the specified parent widget.
    pub fn new(parent: *mut lv_obj_t) -> Self {
        let handle = unsafe { lv_label_create(parent) };
        Label { handle }
    }

    /// Sets the text of the label.
    pub fn text(&self, text: &str) -> &Self {
        let c_str = CString::<20>::from_bytes_with_nul(text.as_bytes()).expect("does not fit into buffer");
        let c_ptr = c_str.as_ptr() as *mut c_char;
        unsafe { lv_label_set_text(self.handle, c_ptr) };
        self
    }

    /// Aligns the label text
    pub fn text_align(&self, text_align: lv_text_align_t) -> &Self {
        unsafe { lv_obj_set_style_text_align(self.handle, text_align, 0) };
        self
    }

    /// Sets the font of the label text.
    pub fn font(&self, font: *const lv_font_t) -> &Self {
        unsafe { lv_obj_set_style_text_font(self.handle, font, 0) };
        self
    }
}

#[derive(Debug, Default)]
pub struct Bar {
    handle: *mut lv_obj_t,
}

impl Widget for Bar {
    fn get_handle(&self) -> *mut lv_obj_t {
        self.handle
    }

    fn set_value(&mut self, value: f32) -> Result<(), Error> {
        unsafe {
            lv_bar_set_value(self.handle, (value*10.) as i32, lv_anim_enable_t_LV_ANIM_ON);
        }
        Ok(())
    }
}

impl Bar {
    pub fn new(parent: *mut lv_obj_t) -> Self {
        let handle = unsafe { lv_bar_create(parent) };
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

    pub fn range(self, from: f32, to: f32) -> Self {
        unsafe { lv_bar_set_range(self.handle, (from*10.) as i32, (to*10.) as i32) };
        self
    }

    pub fn align(self, alignment: lv_align_t, x_offset: i32, y_offset: i32) -> Self {
        unsafe {
            lv_obj_align(self.handle, alignment, x_offset as lv_coord_t, y_offset as lv_coord_t);
        };
        self
    }

}
