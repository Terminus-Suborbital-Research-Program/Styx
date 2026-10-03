#![warn(missing_docs)]
use ::rp235x_hal::{
    i2c::{Controller, Peripheral, I2C},
    Timer,
};
use rp235x_hal::i2c::I2CMode;

#[allow(dead_code)]
pub mod pins {
    use rp235x_hal::{
        gpio::{
            bank0::{Gpio16, Gpio24, Gpio25},
            FunctionI2C, FunctionI2c, Pin, PullNone,
        },
        i2c::{I2CMode, Peripheral},
        pac::I2C0,
    };

    pub type I2cSda = Pin<Gpio24, FunctionI2C, PullNone>;
    pub type I2cScl = Pin<Gpio25, FunctionI2C, PullNone>;
    pub type JupiterI2c = rp235x_hal::i2c::I2C<I2C0, (I2cSda, I2cScl), Peripheral>;
}
