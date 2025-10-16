use crate::app::shared::PpsRunningMode;
use embedded_hal::i2c::{Error as I2cError, ErrorKind as I2cErrorKind};
use esp_hal::i2c::master::I2c;
use esp_hal::Async;
use num_traits::FromPrimitive;
use thiserror_no_std::Error;

#[allow(dead_code)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Error)]
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
    FormatError(),

    I2c(I2cErrorKind),

//    I2cMasterError(#[from] esp_hal::i2c::master::Error),
}

impl<E> From<E> for Error
where
    E: I2cError,
{
    fn from(error: E) -> Self {
        Self::I2c(error.kind())
    }
}

type I2cType = I2c<'static, Async>;

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

    fn get_read_command(&self) -> (u8, usize) {
        match self {
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
        }
    }

    fn evaluate_result(&self, buffer: &[u8;4]) -> Result<ReadResult, Error> {
        match self {
            ReadCommand::ModuleId => Ok(ReadResult::ModuleId((buffer[1] as u16) << 8 | buffer[0] as u16)),
            ReadCommand::GetRunningMode => Ok(ReadResult::RunningMode(
                PpsRunningMode::from_u8(buffer[0]).ok_or(Error::Unknown)?,
            )),
            ReadCommand::ReadbackVoltage => Ok(ReadResult::ReadbackVoltage(f32::from_le_bytes(*buffer))),
            ReadCommand::ReadbackCurrent => Ok(ReadResult::ReadbackCurrent(f32::from_le_bytes(*buffer))),
            ReadCommand::GetTemperature => Ok(ReadResult::Temperature(f32::from_le_bytes(*buffer))),
            ReadCommand::GetInputVoltage => Ok(ReadResult::InputVoltage(f32::from_le_bytes(*buffer))),
            _ => Err(Error::Unsupported),
        }
    }

    #[allow(unused)]
    pub fn receive<I2C: embedded_hal::i2c::I2c>(self, i2c: &mut I2C, address: u8) -> Result<ReadResult, Error> {
        let (cmd, bytes_to_read) = self.get_read_command();
        let mut buffer = [0_u8; 4];
        i2c.write_read(address, &[cmd], &mut buffer[..bytes_to_read])?;
        self.evaluate_result(&buffer)
    }

    pub async fn receive_async(self, i2c: &mut I2cType, address: u8) -> Result<ReadResult, Error> {
        let (cmd, bytes_to_read) = self.get_read_command();
        let mut buffer = [0_u8; 4];
        i2c.write_read_async(address, &[cmd], &mut buffer[..bytes_to_read]).await?;
        self.evaluate_result(&buffer)
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
    fn get_write_command(&self, buffer: &mut [u8;5]) -> usize {
        match self {
            WriteCommand::ModuleEnable(enable) => {
                buffer[0] = 0x04;
                buffer[1] = *enable as u8;
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
        }
    }

    pub fn send<I2C: embedded_hal::i2c::I2c>(self, i2c: &mut I2C, address: u8) -> Result<(), I2C::Error> {
        debug!("send: {:?} to address 0x{:x}", self, address);
        let mut buffer = [0x0_u8; 5];
        let bytes_to_write = self.get_write_command(&mut buffer);
        i2c.write(address, &buffer[..bytes_to_write])
    }

    pub async fn send_async(self, i2c: &mut I2cType, address: u8) -> Result<(), Error> {
        debug!("send: {:?} to address 0x{:x}", self, address);
        let mut buffer = [0x0_u8; 5];
        let bytes_to_write = self.get_write_command(&mut buffer);
        i2c.write_async(address, &buffer[..bytes_to_write]).await?;
        Ok(())
    }
}

pub struct PpsDriver {
    i2c: I2c<'static, Async>,
    address: u8,
}

#[allow(dead_code)]
impl PpsDriver {
    pub fn new(i2c: I2c<'static, Async>, address: u8) -> Result<Self, Error> {
        let mut s = Self { i2c, address };
        s.enable_bl(false)?; // ensure to disable the module ASAP, as we might come from panic reset
        Ok(s)
    }

    pub async fn set_current(&mut self, current: f32) -> Result<&mut Self, Error> {
        let cmd = WriteCommand::SetCurrent(current);
        warn!("set current {}, sending command: {:?}", current, cmd);
        cmd.send_async(&mut self.i2c, self.address).await?;
        Ok(self)
    }

    pub async fn set_voltage(&mut self, voltage: f32) -> Result<&mut Self, Error> {
        let cmd = WriteCommand::SetVoltage(voltage);
        cmd.send_async(&mut self.i2c, self.address).await?;
        Ok(self)
    }

    pub fn enable_bl(&mut self, enabled: bool) -> Result<&mut Self, Error> {
        let cmd = WriteCommand::ModuleEnable(enabled);
        cmd.send(&mut self.i2c, self.address)?;
        Ok(self)
    }

    pub async fn enable(&mut self, enabled: bool) -> Result<&mut Self, Error> {
        let cmd = WriteCommand::ModuleEnable(enabled);
        cmd.send_async(&mut self.i2c, self.address).await?;
        Ok(self)
    }

    pub async fn get_running_mode(&mut self) -> Result<PpsRunningMode, Error> {
        match ReadCommand::GetRunningMode.receive_async(&mut self.i2c, self.address).await? {
            ReadResult::RunningMode(mode) => Ok(mode),
            _ => Err(Error::ResultInvalid),
        }
    }

    pub async fn get_voltage(&mut self) -> Result<f32, Error> {
        match ReadCommand::ReadbackVoltage.receive_async(&mut self.i2c, self.address).await? {
            ReadResult::ReadbackVoltage(voltage) => Ok(voltage),
            _ => Err(Error::ResultInvalid),
        }
    }

    pub async fn get_current(&mut self) -> Result<f32, Error> {
        match ReadCommand::ReadbackCurrent.receive_async(&mut self.i2c, self.address).await? {
            ReadResult::ReadbackCurrent(current) => Ok(current),
            _ => Err(Error::ResultInvalid),
        }
    }

    pub async fn get_temperature(&mut self) -> Result<f32, Error> {
        match ReadCommand::GetTemperature.receive_async(&mut self.i2c, self.address).await? {
            ReadResult::Temperature(temp) => Ok(temp),
            _ => Err(Error::ResultInvalid),
        }
    }

    pub async fn get_input_voltage(&mut self) -> Result<f32, Error> {
        match ReadCommand::GetInputVoltage.receive_async(&mut self.i2c, self.address).await? {
            ReadResult::InputVoltage(voltage) => Ok(voltage),
            _ => Err(Error::ResultInvalid),
        }
    }

    pub async fn get_module_id(&mut self) -> Result<u16, Error> {
        match ReadCommand::ModuleId.receive_async(&mut self.i2c, self.address).await? {
            ReadResult::ModuleId(id) => Ok(id),
            _ => Err(Error::ResultInvalid),
        }
    }
}
