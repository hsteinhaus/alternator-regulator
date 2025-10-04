use static_cell::StaticCell;
use embedded_graphics::prelude::*;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle, Triangle};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    delay::Delay,
    gpio::Output, Async,
    spi::master::SpiDmaBus
};
use mipidsi::{interface::SpiInterface, models::ILI9342CRgb565, Builder};

pub type DisplayInterface<'a> = SpiInterface<'a, ExclusiveDevice<SpiDmaBus<'a, Async>, Output<'a>, Delay>, Output<'a>>;
pub type DisplaySpiDevice<'d> = ExclusiveDevice<SpiDmaBus<'d, Async>, Output<'d>, Delay>;

type D = mipidsi::Display<
    DisplayInterface<'static>,
    ILI9342CRgb565,
    Output<'static>,
>;


#[allow(dead_code)]
pub struct DisplayDriver {
    bl_pin: Output<'static>,
    pub display: &'static mut D,
}

impl DisplayDriver {
    pub fn bl_on(&mut self) {
        self.bl_pin.set_high();
    }

    pub fn bl_off(&mut self) {
        self.bl_pin.set_low();
    }
}

impl DisplayDriver {
    pub fn new(spi_device: DisplaySpiDevice<'static>, mut bl: Output<'static>, mut rst: Output<'static>, dc: Output<'static>) -> Self
    {
        static BUFFER: StaticCell<[u8;128]> = StaticCell::new();
        let buf: &'static mut [u8] = BUFFER.init([0_u8; 128]);
        let di = SpiInterface::new(spi_device, dc, buf);

        bl.set_low();      // avaoid flickering
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
                .unwrap()
        );
        display.clear(Rgb565::BLACK).unwrap();
        Self { bl_pin: bl, display }
    }

    #[allow(dead_code)]
    pub fn draw_smiley(&mut self) -> Result<(), <D as DrawTarget>::Error> {
        // Draw the left eye as a circle located at (50, 100), with a diameter of 40, filled with white
        Circle::new(Point::new(50, 100), 40)
            .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
            .draw(self.display)?;

        // Draw the right eye as a circle located at (50, 200), with a diameter of 40, filled with white
        Circle::new(Point::new(50, 200), 40)
            .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
            .draw(self.display)?;

        // Draw an upside down red triangle to represent a smiling mouth
        Triangle::new(Point::new(130, 140), Point::new(130, 200), Point::new(160, 170))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
            .draw(self.display)?;

        // Cover the top part of the mouth with a black triangle so it looks closed instead of open
        Triangle::new(Point::new(130, 150), Point::new(130, 190), Point::new(150, 170))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(self.display)?;

        Ok(())
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
