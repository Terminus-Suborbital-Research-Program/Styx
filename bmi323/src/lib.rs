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
pub type Reset = u16;
pub type Register = (Address, Reset);
pub const CHIP_ID: Register = (0x00,0x0043);
pub const ERR_REG: Register = (0x01,0x0000);
pub const STATUS: Register = (0x02,0x0001);
pub const ACC_DATA_X: Register = (0x03,0x8000);
pub const ACC_DATA_Y: Register = (0x04,0x8000);
pub const ACC_DATA_Z: Register = (0x05,0x8000);
pub const GYR_DATA_X: Register = (0x06,0x8000);
pub const GYR_DATA_Y: Register = (0x07,0x8000);
pub const GYR_DATA_Z: Register = (0x08,0x8000);
pub const TEMP_DATA: Register = (0x09,0x8000);
pub const SENSOR_TIME_: Register = (0xDA,0x0000);
pub const SENSOR_TIME_1: Register = (0x0E,0x0000);
pub const SAT_FLAGS: Register = (0xDC,0x0000);
pub const INT_STATUS_INT1: Register = (0xDD,0x0000);
pub const INT_STATUS_INT2: Register = (0x0E,0x0000);
pub const INT_STATUS_IBI: Register = (0x0F,0x0000);
pub const FEATURE_I00: Register = (0x10,0x0000);
pub const FEATURE_I01: Register = (0x11,0x0000);
pub const FEATURE_I02: Register = (0x12,0x0000);
pub const FEATURE_I03: Register = (0x13,0x0000);
pub const FEATURE_IO_STATUS: Register = (0x14,0x0018);
pub const FIFO_FILL_LEVEL: Register = (0x15,0x0000);
pub const FIFO_DATA: Register = (0x16,0x0000);
pub const ACC_CONF: Register = (0x20,0x0028);
pub const GYR_CONF: Register = (0x21,0x0048);
pub const ALT_ACC_CONF: Register = (0x28,0x3206);
pub const ALT_GYR_CONF: Register = (0x29,0x1206);
pub const ALT_CONF: Register = (0x2A,0x0000);
pub const ALT_STATUS: Register = (0x2B,0x0000);
pub const FIFO_WATERMARK: Register = (0x35,0x0000);
pub const FIFO_CONF: Register = (0x36,0x0000);
pub const FIFO_CTRL: Register = (0x37,0x0000);
pub const IO_INT_CTRL: Register = (0x38,0x0000);
pub const INT_CONF: Register = (0x39,0x0000);
pub const INT_MAP1: Register = (0x3A,0x0000);
pub const INT_MAP2: Register = (0x3B,0x0000);
pub const FEATURE_CTRL: Register = (0x40,0x0000);
pub const FEATURE_DATA_ADDR: Register = (0x41,0x0000);
pub const FEATURE_DATA_TX: Register = (0x42,0x0000);
pub const FEATURE_DATA_STATUS: Register = (0x43,0x0000);
pub const FEATURE_ENGINE_STATUS: Register = (0x45,0x0000);
pub const FEATURE_EVENT_EXT: Register = (0x47,0x0000);
pub const IO_PDN_CTRL: Register = (0x4F,0x0000);
pub const IO_SPI_IF: Register = (0x50,0x0000);
pub const IO_PAD_STRENGTH: Register = (0x51,0x000A);
pub const IO_I2C_IF: Register = (0x52,0x0000);
pub const IO_ODR_DEVIATION: Register = (0x53,0x0000);
pub const ACC_DP_OFF_X: Register = (0x60,0x0000);
pub const ACC_DP_DGAIN_X: Register = (0x61,0x0000);
pub const ACC_DP_OFF_Y: Register = (0x62,0x0000);
pub const ACC_DP_DGAIN_Y: Register = (0x63,0x0000);
pub const ACC_DP_OFF_Z: Register = (0x64,0x0000);
pub const ACC_DP_DGAIN_Z: Register = (0x65,0x0000);
pub const GYR_DP_OFF_X: Register = (0x66,0x0000);
pub const GYR_DP_DGAIN_X: Register = (0x67,0x0000);
pub const GYR_DP_OFF_Y: Register = (0x68,0x0000);
pub const GYR_DP_DGAIN_Y: Register = (0x69,0x0000);
pub const GYR_DP_OFF_Z: Register = (0x6A,0x0000);
pub const GYR_DP_DGAIN_Z: Register = (0x6B,0x0000);
pub const I3C_TC_SYNC_TPH: Register = (0x70,0x0000);
pub const I3C_TC_SYNC_TU: Register = (0x71,0x0000);
pub const I3C_TC_SYNC_ODR: Register = (0x72,0x0000);
pub const CMD: Register = (0x7E,0x0000);
pub const CFG_RES: Register = (0x7F,0x0000);