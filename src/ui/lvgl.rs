use core::ffi::c_char;
use heapless::{format, CString, String};
use heapless::c_string::ExtendError;
use lvgl_rust_sys::*;
use thiserror_no_std::Error;
use crate::fmt::Debug2Format;

#[derive(Error, Debug)]
pub enum WidgetError {
    #[error(transparent)]
    FormatError(#[from] core::fmt::Error),

    #[error("LVGL: got NULL pointer")]
    LvglNullPointer,

    #[error("CString error")]
    ExtendError(#[from] ExtendError),
}

#[cfg(feature = "defmt")]
impl defmt::Format for WidgetError {
    fn format(&self, f: defmt::Formatter) {
        match self {
            WidgetError::FormatError(fe) => defmt::write!(f, "FormatError: {:?}", Debug2Format(&fe)),
            WidgetError::LvglNullPointer => defmt::write!(f, "Got NULL pointer from LVGL"),
            WidgetError::ExtendError(ee) => defmt::write!(f, "Could not extend C string: {:?}", Debug2Format(&ee)),
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

    fn set_value(&mut self, value: f32) -> Result<(), WidgetError>;
}

#[derive(Debug, Default)]
pub struct Meter<'a> {
    handle: *mut lv_obj_t,
    current_needle: *mut lv_meter_indicator_t,
    rpm_needle: *mut lv_meter_indicator_t,
    current_label: Label<'a>,
    state_label: Label<'a>,
}

impl<'a> Widget for Meter<'a> {
    fn get_handle(&self) -> *mut lv_obj_t {
        self.handle
    }

    fn set_value(&mut self, value: f32) -> Result<(), WidgetError> {
        unsafe {
            lv_meter_set_indicator_value(self.handle, self.current_needle, value as i32);
        }
        self.current_label.set_value(value)?;
        Ok(())
    }
}

impl<'a> Meter<'a> {
    pub fn new(parent: *mut lv_obj_t) -> Result<Self, WidgetError> {
        unsafe {
            let meter = lv_meter_create(parent);
            lv_obj_align(meter, LV_ALIGN_CENTER as lv_align_t, 0, 0);
            lv_obj_set_width(meter, 228);
            lv_obj_set_height(meter, 228);

            let scale = lv_meter_add_scale(meter).as_mut().ok_or(WidgetError::LvglNullPointer)?;
            scale.min = 0;
            scale.max = 150;
            scale.angle_range = 240;
            scale.rotation = 150;

            let current_needle = lv_meter_add_needle_line(meter, scale, 5, lv_color_hex(0xff0000), -4)
                .as_mut()
                .ok_or(WidgetError::LvglNullPointer)?;
            let rpm_needle = lv_meter_add_needle_line(meter, scale, 3, lv_color_hex(0xaaaaaa), -30)
                .as_mut()
                .ok_or(WidgetError::LvglNullPointer)?;

            let green_arc = lv_meter_add_arc(meter, scale, 10, lv_color_hex(0x009f00), 10)
                .as_mut()
                .ok_or(WidgetError::LvglNullPointer)?;
            green_arc.start_value = 0;
            green_arc.end_value = 99;

            let yellow_arc = lv_meter_add_arc(meter, scale, 10, lv_color_hex(0xffff00), 10)
                .as_mut()
                .ok_or(WidgetError::LvglNullPointer)?;
            yellow_arc.start_value = 100;
            yellow_arc.end_value = 119;

            let red_arc = lv_meter_add_arc(meter, scale, 10, lv_color_hex(0xff0000), 10)
                .as_mut()
                .ok_or(WidgetError::LvglNullPointer)?;
            red_arc.start_value = 120;
            red_arc.end_value = 150;

            let scale2 = lv_meter_add_scale(meter).as_mut().ok_or(WidgetError::LvglNullPointer)?;
            scale2.min = 0;
            scale2.max = 150;
            scale2.angle_range = 240;
            scale2.rotation = 150;
            scale2.tick_width = 1;
            scale2.tick_cnt = 51;
            scale2.tick_length = 10;
            scale2.tick_color = lv_color_hex(0x000000);
            scale2.tick_major_nth = 5;
            scale2.tick_major_width = 2;
            scale2.tick_major_length = 10;
            scale2.tick_major_color = lv_color_hex(0x404040);
            scale2.label_gap = 10;

            lv_obj_set_style_text_font(meter, &lv_font_montserrat_14, 0);
            lv_meter_set_indicator_value(meter, current_needle, 42);

            // Create labels for the meter
            let current_label = Label::new(meter, "")?;
            current_label
                .text("-.-")?
                .align(LV_ALIGN_CENTER as lv_align_t, 0, 50)
                .font(&lv_font_montserrat_40);

            let _static_label = Label::new(meter, "")?
                .text("Amps")?
                .align(LV_ALIGN_CENTER as lv_align_t, 0, 75)
                .font(&lv_font_montserrat_14);

            let state_label = Label::new(meter, "")?;
            state_label
                .text("<unknown>")?
                .align(LV_ALIGN_CENTER as lv_align_t, 0, -35);

            Ok(Meter {
                handle: meter,
                current_label,
                current_needle,
                rpm_needle,
                state_label,
            })
        }
    }

    pub fn set_state(&mut self, state: &str) -> Result<&Self, WidgetError> {
        self.state_label.text(state)?;
        Ok(self)
    }

    pub fn set_rpm(&mut self, rpm: f32) -> Result<&Self, WidgetError> {
        unsafe {
            lv_meter_set_indicator_value(self.handle, self.rpm_needle, (rpm / 100.) as i32);
        }
        Ok(self)
    }
}

#[derive(Debug, Default)]
pub struct Label<'a> {
    handle: *mut lv_obj_t,
    unit: &'a str,
}

impl<'a> Widget for Label<'a> {
    fn get_handle(&self) -> *mut lv_obj_t {
        self.handle
    }

    fn set_value(&mut self, value: f32) -> Result<(), WidgetError> {
        let s: String<10> = format!("{:.1}{}", value, self.unit)?;
        self.text(s.as_str())?;
        Ok(())
    }
}

impl<'a> Label<'a> {
    /// Creates a new `Label` on the specified parent widget.
    pub fn new(parent: *mut lv_obj_t, unit: &'a str) -> Result<Self, WidgetError> {
        let handle = unsafe { lv_label_create(parent) };
        if handle.is_null() {
            Err(WidgetError::LvglNullPointer)
        }
        else {
            Ok(Label { handle, unit })
        }
    }

    /// Sets the text of the label.
    pub fn text(&self, text: &str) -> Result<&Self, WidgetError> {
        let c_str = CString::<20>::from_bytes_with_nul(text.as_bytes())?;
        let c_ptr = c_str.as_ptr() as *mut c_char;
        unsafe { lv_label_set_text(self.handle, c_ptr) };
        Ok(self)
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

    fn set_value(&mut self, value: f32) -> Result<(), WidgetError> {
        unsafe {
            lv_bar_set_value(self.handle, (value * 10.) as i32, lv_anim_enable_t_LV_ANIM_OFF);
        }
        Ok(())
    }
}

impl Bar {
    pub fn new(parent: *mut lv_obj_t) -> Result<Self, WidgetError> {
        let handle = unsafe { lv_bar_create(parent) };
        if handle.is_null() {
            Err(WidgetError::LvglNullPointer)
        }
        else {
            Ok(Bar { handle })
        }
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
        unsafe { lv_bar_set_range(self.handle, (from * 10.) as i32, (to * 10.) as i32) };
        self
    }

    pub fn align(self, alignment: lv_align_t, x_offset: i32, y_offset: i32) -> Self {
        unsafe {
            lv_obj_align(self.handle, alignment, x_offset as lv_coord_t, y_offset as lv_coord_t);
        };
        self
    }
}
