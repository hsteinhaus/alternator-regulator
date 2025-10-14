use crate::app::shared::PpsRunningMode;
use embedded_hal::i2c::{Error as I2cError, ErrorKind as I2cErrorKind};
use esp_hal::i2c::master::I2c;
use esp_hal::Async;
use num_traits::FromPrimitive;

#[allow(dead_code)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug)]
pub enum Error {
    /// some other error
    Unknown,

    /// invalid result
    ResultInvalid,

    /// unsupported
    Unsupported,

    /// PPS module not present
    ModuleNotFound,

    /// An error in the  underlying I²C system
    I2c(I2cErrorKind),
}

impl<E> From<E> for Error
where
    E: I2cError,
{
    fn from(error: E) -> Self {
        Self::I2c(error.kind())
    }
}

#[allow(dead_code)]
enum ReadCommand {
    ModuleId,
    GetRunningMode,
    GetDataFlag,
    ReadbackVoltage,
    ReadbackCurrent,
    GetTemperature,
    GetInputVoltage,
    GetAddress,
    PsuUidW0,
    PsuUidW1,
    PsuUidW2,
}

pub enum ReadResult {
    ModuleId(u16),
    RunningMode(PpsRunningMode),
    ReadbackVoltage(f32),
    ReadbackCurrent(f32),
    Temperature(f32),
    InputVoltage(f32),
}

impl ReadCommand {
    pub fn receive<I2C: embedded_hal::i2c::I2c>(self, i2c: &mut I2C, address: u8) -> Result<ReadResult, Error> {
        let (cmd, bytes_to_read) = match self {
            ReadCommand::ModuleId => (0x0, 2),
            ReadCommand::GetRunningMode => (0x05, 1),
            ReadCommand::GetDataFlag => (0x07, 1),
            ReadCommand::ReadbackVoltage => (0x08, 4),
            ReadCommand::ReadbackCurrent => (0x0c, 4),
            ReadCommand::GetTemperature => (0x10, 4),
            ReadCommand::GetInputVoltage => (0x14, 4),
            ReadCommand::GetAddress => (0x50, 1),
            ReadCommand::PsuUidW0 => (0x52, 4),
            ReadCommand::PsuUidW1 => (0x56, 4),
            ReadCommand::PsuUidW2 => (0x5a, 4),
        };
        let mut buffer = [0_u8; 4];
        i2c.write_read(address, &[cmd], &mut buffer[..bytes_to_read])?;
        match self {
            ReadCommand::ModuleId => Ok(ReadResult::ModuleId((buffer[1] as u16) << 8 | buffer[0] as u16)),
            ReadCommand::GetRunningMode => Ok(ReadResult::RunningMode(
                PpsRunningMode::from_u8(buffer[0]).ok_or(Error::Unknown)?,
            )),
            ReadCommand::ReadbackVoltage => Ok(ReadResult::ReadbackVoltage(f32::from_le_bytes(buffer))),
            ReadCommand::ReadbackCurrent => Ok(ReadResult::ReadbackCurrent(f32::from_le_bytes(buffer))),
            ReadCommand::GetTemperature => Ok(ReadResult::Temperature(f32::from_le_bytes(buffer))),
            ReadCommand::GetInputVoltage => Ok(ReadResult::InputVoltage(f32::from_le_bytes(buffer))),
            _ => Err(Error::Unsupported),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum WriteCommand {
    ModuleEnable(bool),
    SetVoltage(f32),
    SetCurrent(f32),
}

impl WriteCommand {
    pub fn send<I2C: embedded_hal::i2c::I2c>(self, i2c: &mut I2C, address: u8) -> Result<(), I2C::Error> {
        debug!("send: {:?} to address 0x{:x}", self, address);
        let mut buffer = [0x0_u8; 5];
        let bytes_to_write = match self {
            WriteCommand::ModuleEnable(enable) => {
                buffer[0] = 0x04;
                buffer[1] = enable as u8;
                2
            }
            WriteCommand::SetVoltage(voltage) => {
                buffer[0] = 0x18;
                buffer[1..].copy_from_slice(voltage.to_le_bytes().as_slice());
                5
            }
            WriteCommand::SetCurrent(current) => {
                buffer[0] = 0x1c;
                buffer[1..].copy_from_slice(current.to_le_bytes().as_slice());
                5
            }
        };
        i2c.write(address, &buffer[..bytes_to_write])
    }
}

pub struct PpsDriver {
    i2c: I2c<'static, Async>,
    address: u8,
}

#[allow(dead_code)]
impl PpsDriver {
    pub fn new(i2c: I2c<'static, Async>, address: u8) -> Result<Self, Error> {
        Ok(Self { i2c, address })
    }

    pub fn set_current(&mut self, current: f32) -> Result<&mut Self, Error> {
        let cmd = WriteCommand::SetCurrent(current);
        cmd.send(&mut self.i2c, self.address)?;
        Ok(self)
    }

    pub fn set_voltage(&mut self, voltage: f32) -> Result<&mut Self, Error> {
        let cmd = WriteCommand::SetVoltage(voltage);
        cmd.send(&mut self.i2c, self.address)?;
        Ok(self)
    }

    pub fn enable(&mut self, enabled: bool) -> Result<&mut Self, Error> {
        let cmd = WriteCommand::ModuleEnable(enabled);
        cmd.send(&mut self.i2c, self.address)?;
        Ok(self)
    }

    pub fn get_running_mode(&mut self) -> Result<PpsRunningMode, Error> {
        match ReadCommand::GetRunningMode.receive(&mut self.i2c, self.address)? {
            ReadResult::RunningMode(mode) => Ok(mode),
            _ => Err(Error::ResultInvalid),
        }
    }

    pub fn get_voltage(&mut self) -> Result<f32, Error> {
        match ReadCommand::ReadbackVoltage.receive(&mut self.i2c, self.address)? {
            ReadResult::ReadbackVoltage(voltage) => Ok(voltage),
            _ => Err(Error::ResultInvalid),
        }
    }

    pub fn get_current(&mut self) -> Result<f32, Error> {
        match ReadCommand::ReadbackCurrent.receive(&mut self.i2c, self.address)? {
            ReadResult::ReadbackCurrent(current) => Ok(current),
            _ => Err(Error::ResultInvalid),
        }
    }

    pub fn get_temperature(&mut self) -> Result<f32, Error> {
        match ReadCommand::GetTemperature.receive(&mut self.i2c, self.address)? {
            ReadResult::Temperature(temp) => Ok(temp),
            _ => Err(Error::ResultInvalid),
        }
    }

    pub fn get_input_voltage(&mut self) -> Result<f32, Error> {
        match ReadCommand::GetInputVoltage.receive(&mut self.i2c, self.address)? {
            ReadResult::InputVoltage(voltage) => Ok(voltage),
            _ => Err(Error::ResultInvalid),
        }
    }

    pub fn get_module_id(&mut self) -> Result<u16, Error> {
        match ReadCommand::ModuleId.receive(&mut self.i2c, self.address)? {
            ReadResult::ModuleId(id) => Ok(id),
            _ => Err(Error::ResultInvalid),
        }
    }
}
