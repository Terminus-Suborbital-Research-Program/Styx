#![no_std]
// Library
#[cfg(feature = "sync")]
pub mod sync_mod;
#[cfg(feature = "sync")]
pub use sync_mod::*;
#[cfg(feature = "async")]
pub mod async_mod;
#[cfg(feature = "async")]
pub use async_mod::*;

// Types/Constants
pub type Address = u8;
pub type Reset = u8;
pub type Register = (Address, Reset);
pub const CHIP_ID: Register = (0x00, 0x33);
pub const ERR_REG: Register = (0x02, 0x00);
pub const PAD_CTRL: Register = (0x03, 0x07);
pub const PMU_CMD_AGGR_SET: Register = (0x04, 0x14);
pub const PMU_CMD_AXIS_EN: Register = (0x05, 0x07);
pub const PMU_CMD: Register = (0x06, 0x00);
pub const PMU_CMD_STATUS_0: Register = (0x07, 0x00);
pub const PMU_CMD_STATUS_1: Register = (0x08, 0x00);
pub const I3C_ERR: Register = (0x09, 0x00);
pub const I2C_WDT_SET: Register = (0x0A, 0x00);
pub const INT_CTRL: Register = (0x2E, 0x00);
pub const INT_CTRL_IBI: Register = (0x2F, 0x00);
pub const INT_STATUS: Register = (0x30, 0x00);
pub const MAG_X_XLSB: Register = (0x31, 0x7F);
pub const MAG_X_LSB: Register = (0x32, 0x7F);
pub const MAG_X_MSB: Register = (0x33, 0x7F);
pub const MAG_Y_XLSB: Register = (0x34, 0x7F);
pub const MAG_Y_LSB: Register = (0x35, 0x7F);
pub const MAG_Y_MSB: Register = (0x36, 0x7F);
pub const MAG_Z_XLSB: Register = (0x37, 0x7F);
pub const MAG_Z_LSB: Register = (0x38, 0x7F);
pub const MAG_Z_MSB: Register = (0x39, 0x7F);
pub const TEMP_XLSB: Register = (0x3A, 0x7F);
pub const TEMP_LSB: Register = (0x3B, 0x7F);
pub const TEMP_MSB: Register = (0x3C, 0x7F);
pub const SENSORTIME_XLSB: Register = (0x3D, 0x7F);
pub const SENSORTIME_LSB: Register = (0x3E, 0x7F);
pub const SENSORTIME_MSB: Register = (0x3F, 0x7F);
pub const OTP_CMD_REG: Register = (0x50, 0x00);
pub const OTP_DATA_MSB_REG: Register = (0x52, 0x00);
pub const OTP_DATA_LSB_REG: Register = (0x53, 0x00);
pub const OTP_STATUS_REG: Register = (0x55, 0x10);
pub const TMR_SELFTEST_USER: Register = (0x60, 0x60);
pub const CTRL_USER: Register = (0x61, 0x00);
pub const CMD: Register = (0x7E, 0x00);
