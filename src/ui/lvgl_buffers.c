#include <lvgl.h>

#define SCREEN_WIDTH  320
#define SCREEN_HEIGHT 240
#define COLOR_BUF_LINES  40

static lv_disp_draw_buf_t draw_buf __attribute__ ((section (".dram2_uninit")));
static lv_color_t color_buf[ SCREEN_WIDTH * COLOR_BUF_LINES] __attribute__ ((section (".dram2_uninit")));

static lv_disp_drv_t disp_drv;

void lvgl_disp_init(void(*flush_cb)( lv_disp_drv_t*, const lv_area_t *, lv_color_t *), void* user_data) {
    lv_disp_draw_buf_init( &draw_buf, color_buf, NULL, SCREEN_WIDTH * COLOR_BUF_LINES);

    /*Initialize the display*/
    lv_disp_drv_init( &disp_drv );
    /*Change the following line to your display resolution*/
    disp_drv.hor_res = SCREEN_WIDTH;
    disp_drv.ver_res = SCREEN_HEIGHT;
    disp_drv.flush_cb = flush_cb;
    disp_drv.draw_buf = &draw_buf;
    disp_drv.user_data = user_data;
    lv_disp_drv_register( &disp_drv );
 }