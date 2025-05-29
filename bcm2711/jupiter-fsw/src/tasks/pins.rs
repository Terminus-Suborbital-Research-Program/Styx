use std::{
    sync::{Arc, Mutex, Once},
    time::Duration,
};

use i2cdev::{
    core::I2CDevice as _,
    linux::{LinuxI2CDevice, LinuxI2CError},
};
use log::{debug, warn};

use common::{
    battery_state::BatteryState,
    indicators::{IndicatorStates, MalformedIndicatorError},
};

static ATMEGA_ONCE: Once = Once::new();

/// ATMega abstraction
pub struct Atmega {
    device: LinuxI2CDevice,
}

impl From<LinuxI2CDevice> for Atmega {
    fn from(device: LinuxI2CDevice) -> Self {
        Self { device }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum IndicatorError {
    I2CError(LinuxI2CError),
    Malformed(MalformedIndicatorError),
}

impl From<LinuxI2CError> for IndicatorError {
    fn from(err: LinuxI2CError) -> Self {
        IndicatorError::I2CError(err)
    }
}
impl From<MalformedIndicatorError> for IndicatorError {
    fn from(err: MalformedIndicatorError) -> Self {
        IndicatorError::Malformed(err)
    }
}

impl Atmega {
    pub fn new(device: LinuxI2CDevice) -> Self {
        device.into()
    }

    pub fn pins(&mut self) -> Result<IndicatorStates, IndicatorError> {
        Ok(IndicatorStates::try_from(self.device.smbus_read_byte()?)?)
    }

    /// Write one byte to register 0x00 (SMBus “command” 0x00).
    fn write_reg0(&mut self, value: u8) -> Result<(), IndicatorError> {
        // LinuxI2CDevice already has smbus_write_byte_data(cmd, value)
        self.device
            .smbus_write_byte_data(0x00, value)
            .map_err(IndicatorError::from)
    }

    /// Write a battery state to the device
    pub fn set_battery_latch(&mut self, latch_state: BatteryState) -> Result<(), IndicatorError> {
        self.write_reg0(latch_state.into())
    }
}
