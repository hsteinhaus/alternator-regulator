#include <lvgl.h>

#define screenWidth  320
#define screenHeight 240

static lv_disp_draw_buf_t draw_buf;
static lv_color_t buf[ screenWidth * 10];
static lv_disp_drv_t disp_drv;

void lvgl_disp_init(void(*flush_cb)( lv_disp_drv_t*, const lv_area_t *, lv_color_t *), void* user_data) {
    lv_disp_draw_buf_init( &draw_buf, buf, NULL, screenWidth *10);

    /*Initialize the display*/
    lv_disp_drv_init( &disp_drv );
    /*Change the following line to your display resolution*/
    disp_drv.hor_res = screenWidth;
    disp_drv.ver_res = screenHeight;
    disp_drv.flush_cb = flush_cb;
    disp_drv.draw_buf = &draw_buf;
    disp_drv.user_data = user_data;
    lv_disp_drv_register( &disp_drv );
 }