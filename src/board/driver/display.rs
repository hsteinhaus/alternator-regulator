use crate::board::startup::SpiDeviceType;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use esp_hal::{delay::Delay, gpio::Output};
use mipidsi::{interface::SpiInterface, models::ILI9342CRgb565, Builder};
use static_cell::StaticCell;

type DisplayInterface<'a> = SpiInterface<'a, SpiDeviceType, Output<'a>>;
type D = mipidsi::Display<DisplayInterface<'static>, ILI9342CRgb565, Output<'static>>;

#[allow(dead_code)]
pub struct DisplayDriver {
    bl_pin: Output<'static>,
    pub display: &'static mut D,
}

impl DisplayDriver {
    pub fn bl_on(&mut self) {
        self.bl_pin.set_high();
    }

    #[allow(dead_code)]
    pub fn bl_off(&mut self) {
        self.bl_pin.set_low();
    }
}

impl DisplayDriver {
    pub fn new(
        spi_device: SpiDeviceType,
        mut bl: Output<'static>,
        mut rst: Output<'static>,
        dc: Output<'static>,
    ) -> Self {
        static BUFFER: StaticCell<[u8; 128]> = StaticCell::new();
        let di_buf: &'static mut [u8] = BUFFER.init([0_u8; 128]);
        let di = SpiInterface::new(spi_device, dc, di_buf);

        bl.set_low(); // avaoid flickering
        rst.set_high();
        let mut delay = Delay::new();
        static DISPLAY: StaticCell<D> = StaticCell::new();
        let display = DISPLAY.init(
            Builder::new(ILI9342CRgb565, di)
                .invert_colors(mipidsi::options::ColorInversion::Inverted)
                .color_order(mipidsi::options::ColorOrder::Bgr)
                .display_size(320, 240)
                .reset_pin(rst)
                .init(&mut delay)
                .unwrap(),
        );
        display.clear(Rgb565::BLACK).unwrap();
        Self { bl_pin: bl, display }
    }
}

impl DrawTarget for DisplayDriver {
    type Color = Rgb565;
    type Error = <D as DrawTarget>::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        // Forward the draw_iter call to the `display` implementation
        self.display.draw_iter(pixels)
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        // Forward the fill_solid call to the `display` implementation
        self.display.fill_solid(area, color)
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        // Forward the fill_contiguous call to the `display` implementation
        self.display.fill_contiguous(area, colors)
    }
}

// Ensure we support getting the size of the display
impl OriginDimensions for DisplayDriver {
    fn size(&self) -> Size {
        // Forward the size computation to the `display` implementation
        self.display.size()
    }
}
