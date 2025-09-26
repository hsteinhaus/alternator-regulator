use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle, Triangle};
//use esp_hal::peripherals::DMA_SPI2;

use embedded_hal_bus::spi::ExclusiveDevice;

use esp_hal::{delay::Delay, gpio::Output, spi::master::Spi, Async};
use mipidsi::{interface::SpiInterface, models::ILI9342CRgb565, Builder};
use static_cell::StaticCell;

//pub type DisplayDmaChannel<'a> = DMA_SPI2<'a>;
pub type DisplayInterface<'a> = SpiInterface<'static, DisplaySpi<'a>, Output<'static>>;
//pub type DisplaySpi<'d> = ExclusiveDevice<SpiDmaBus<'d, Async>, DummyOutputPin, Delay>;
pub type DisplaySpi<'d> = ExclusiveDevice<Spi<'d, Async>, Output<'static>, Delay>;

//type DisplayType = mipidsi::Display<SpiInterface<'static, ExclusiveDevice<SPI, Output<'static>, NoDelay>, Output<'static>>, ILI9342CRgb565, Output<'static>>;

type D = mipidsi::Display<
    SpiInterface<
        'static,
        ExclusiveDevice<Spi<'static, Async>, Output<'static>, Delay>,
        Output<'static>,
    >,
    ILI9342CRgb565,
    Output<'static>,
>;

#[allow(dead_code)]
pub struct DisplayDriver {
    bl_pin: Output<'static>,
    pub display: &'static mut D,
}

impl DisplayDriver {
    pub fn new(
        spi: DisplayInterface<'static>,
        mut bl: Output<'static>,
        mut rst: Output<'static>,
    ) -> Self {
        let mut delay = Delay::new();
        //let buffer = Box::leak(Box::new([0_u8; 512]));

        rst.set_high();
        static DISPLAY: StaticCell<D> = StaticCell::new();
        let display = DISPLAY.init(
            Builder::new(ILI9342CRgb565, spi)
                .invert_colors(mipidsi::options::ColorInversion::Inverted)
                .color_order(mipidsi::options::ColorOrder::Bgr)
                .display_size(320, 240)
                .reset_pin(rst)
                .init(&mut delay)
                .unwrap(),
        );
        display.clear(Rgb565::BLACK).unwrap();
        bl.set_high();

        Self {
            bl_pin: bl,
            display,
        }
    }

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
        Triangle::new(
            Point::new(130, 140),
            Point::new(130, 200),
            Point::new(160, 170),
        )
        .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
        .draw(self.display)?;

        // Cover the top part of the mouth with a black triangle so it looks closed instead of open
        Triangle::new(
            Point::new(130, 150),
            Point::new(130, 190),
            Point::new(150, 170),
        )
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

    fn fill_solid(
        &mut self,
        area: &Rectangle,
        color: Self::Color,
    ) -> Result<(), Self::Error> {
        // Forward the fill_solid call to the `display` implementation
        self.display.fill_solid(area, color)
    }

    fn fill_contiguous<I>(
        &mut self,
        area: &Rectangle,
        colors: I,
    ) -> Result<(), Self::Error>
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