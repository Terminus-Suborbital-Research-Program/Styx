use embedded_hal_bus::i2c::AtomicDevice;
use mcf8316c_rs::controller::MotorController;
use pins::{AvionicsI2CSclPin, AvionicsI2CSdaPin, EscI2CSclPin, EscI2CSdaPin, LedPin};
use rp235x_hal::{
    async_utils::AsyncPeripheral,
    gpio::{FunctionI2C, FunctionSio, Pin, PullNone, PullUp, SioOutput},
    i2c::Controller,
    pac::{I2C0, I2C1},
    I2C,
};

use crate::peripherals::async_i2c::AsyncI2c;

pub mod pins {
    use rp235x_hal::gpio::bank0::*;

    /// RBF Inhibit pin
    pub type RBFPin = Gpio4;

    /// Flap servo PWM
    pub type RelayServoPWMPin = Gpio1;
    /// Flap servo PWM
    pub type FlapServoPWMPin = Gpio3;

    /// Flab servo mosfet
    pub type FlapMosfetPin = Gpio2;
    /// Relay servo mosfet
    pub type RelayMosfetPin = Gpio0;



    // Mux pins are 14, 13, 11, 10 for S0, S1, S2, S3

    /// Mux S0
    pub type MuxS0Pin = Gpio14;
    /// Mux S1
    pub type MuxS1Pin = Gpio13;
    /// Mux S2
    pub type MuxS2Pin = Gpio11;
    /// Mux S3
    pub type MuxS3Pin = Gpio10;
    /// Mux ADC0
    pub type MuxADCPin = Gpio40;

    /// I2C SDA pin
    pub type AvionicsI2CSdaPin = Gpio6;
    /// I2C SCL pin
    pub type AvionicsI2CSclPin = Gpio7;
    
    /// ESC I2C SDA pin
    pub type EscI2CSdaPin = Gpio18;
    /// ESC I2C SCL pin
    pub type EscI2CSclPin = Gpio19;

    /// Software controlled LED
    pub type LedPin = Gpio27;
}

/// Software-controlled LED
pub type SoftwareLED = Pin<LedPin, FunctionSio<SioOutput>, PullNone>;

// Avionics I2C bus
pub type AvionicsI2cBus = AsyncI2c<
    I2C<
        I2C1,
        (
            Pin<AvionicsI2CSdaPin, FunctionI2C, PullUp>,
            Pin<AvionicsI2CSclPin, FunctionI2C, PullUp>,
        ),
        Controller,
    >,
>;

/// ACS ESC I2C bus
pub type MotorI2cBus = AsyncI2c<
    I2C<
        I2C1,
        (
            Pin<EscI2CSdaPin, FunctionI2C, PullUp>,
            Pin<EscI2CSclPin, FunctionI2C, PullUp>,
        ),
        Controller,
    >,
>;

/// A motor controller on a shared bus
pub type ReactionWheelMotor = ();
