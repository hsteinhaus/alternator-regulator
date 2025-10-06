use crate::board::driver::display::DisplayDriver;
use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::ptr::addr_of_mut;
use defmt::{debug, warn};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use lvgl_rust_sys::*;

const SCREEN_WIDTH: usize = 320;
const SCREEN_HEIGHT: usize = 240;
const COLOR_BUF_LINES: usize = 48;

// buffers and driver struct placed in .dram2_uninit section for resource reasons
#[link_section = ".dram2_uninit"]
static mut DRAW_BUF: MaybeUninit<lv_disp_draw_buf_t> = MaybeUninit::uninit();

#[link_section = ".dram2_uninit"]
static mut COLOR_BUF: MaybeUninit<[lv_color_t; SCREEN_WIDTH * COLOR_BUF_LINES]> = MaybeUninit::uninit();

#[link_section = ".dram2_uninit"]
static mut DISP_DRV: MaybeUninit<lv_disp_drv_t> = MaybeUninit::uninit();

pub fn lvgl_disp_init(user_data: *mut core::ffi::c_void) {
    // Initialize the draw buffer
    unsafe {
        lv_disp_draw_buf_init(
            addr_of_mut!(DRAW_BUF).cast::<lv_disp_draw_buf_t>(),
            addr_of_mut!(COLOR_BUF).cast::<c_void>(),
            core::ptr::null_mut(),
            (SCREEN_WIDTH * COLOR_BUF_LINES) as u32,
        );
    }

    // Initialize the display driver
    unsafe {
        lv_disp_drv_init(addr_of_mut!(DISP_DRV).cast::<lv_disp_drv_t>());

        let disp_drv = &mut *addr_of_mut!(DISP_DRV).cast::<lv_disp_drv_t>();
        disp_drv.hor_res = SCREEN_WIDTH as i16;
        disp_drv.ver_res = SCREEN_HEIGHT as i16;
        disp_drv.flush_cb = Some(flush_cb);
        disp_drv.draw_buf = addr_of_mut!(DRAW_BUF).cast::<lv_disp_draw_buf_t>();
        disp_drv.user_data = user_data;

        lv_disp_drv_register(addr_of_mut!(DISP_DRV).cast::<lv_disp_drv_t>());
    }
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
    } else {
        warn!("disp_drv_p is null");
    }
    lv_disp_flush_ready(disp_drv_p);
    debug!("Flush done");
}
