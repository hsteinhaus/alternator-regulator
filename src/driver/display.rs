use alloc::boxed::Box;

use esp_hal::peripherals::DMA_SPI2;

use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::delay::Delay;
use esp_hal::spi::master::Spi;
use esp_hal::{
    gpio::Output,
    Async,
};

use embedded_hal::digital::OutputPin;

use mipidsi::models::ILI9342CRgb565;
use mipidsi::Builder;
use static_cell::StaticCell;

use mipidsi::interface::SpiInterface;

//pub type DisplayDmaChannel<'a> = DMA_SPI2<'a>;
pub type DisplayInterface<'a> = SpiInterface<'static, DisplaySpi<'a>, Output<'static>>;
//pub type DisplaySpi<'d> = ExclusiveDevice<SpiDmaBus<'d, Async>, DummyOutputPin, Delay>;
pub type DisplaySpi<'d> = ExclusiveDevice<Spi<'d, Async>, Output<'static>, Delay>;

//
// pub struct DummyOutputPin;
// impl DigitalErrorType for DummyOutputPin {
//     type Error = Infallible;
// }
// impl OutputPin for DummyOutputPin {
//     fn set_low(&mut self) -> Result<(), Self::Error> {
//         Ok(())
//     }
//
//     fn set_high(&mut self) -> Result<(), Self::Error> {
//         Ok(())
//     }
// }
//

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

pub struct Display<BL>
where
//D: embedded_graphics::draw_target::DrawTarget
{
    bl_pin: BL,
    pub display: &'static mut D,
}

impl<BL> Display<BL>
where
    BL: OutputPin,
{
    pub fn new(spi: DisplayInterface<'static>, bl: BL, mut rst: Output<'static>) -> Self {
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

        Self {
            bl_pin: bl,
            display,
        }
    }
}
