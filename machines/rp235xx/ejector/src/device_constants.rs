#![warn(missing_docs)]

#![warn(missing_docs)]

use pins::{
    EjectionPin, GreenLedPin, JupiterRxPin, JupiterTxPin, OnboardLEDPin,
   RedLedPin,
};
use rp235x_hal::{
    I2C, Timer, gpio::{FunctionI2C, FunctionSio, Pin, PullDown, PullNone, PullUp, SioInput, SioOutput}, i2c::{Controller, Peripheral}, pac::{I2C0, I2C1, UART0, UART1}, timer::CopyableTimer1, uart::{Enabled, UartPeripheral}
};


#[allow(dead_code)]
pub mod pins {
    use rp235x_hal::gpio::{
        FunctionI2C, FunctionSio, FunctionUart, Pin, PullDown, PullUp, SioInput, SioOutput, bank0::{
            Gpio0, Gpio1, Gpio2, Gpio4, Gpio5, Gpio8, Gpio9, Gpio10, Gpio11, Gpio12, Gpio16, Gpio17, Gpio20, Gpio21, Gpio24, Gpio25, Gpio26, Gpio27, Gpio32, Gpio33
        }
    };

    // Ejector Heartbeat Output
    pub type OnboardLEDPin = Gpio25;

    // Camera Startup should be right but the heartbeat and Cam LED Pins might be wrong
    // (inconsistency in ejector pinout doc) ask Brooks later

    /// Camera GPIO activation
    pub type CamMosfetPin = Pin<Gpio12, FunctionSio<SioOutput>, PullDown>;

    // Camera LED Pin
    pub type RedLedPin = Gpio11;

    // RBF LED PIN
    pub type GreenLedPin = Gpio10;

    // RBF PIN
    pub type RBFPin = Pin<Gpio2, FunctionSio<SioInput>, PullDown>;

    /// Ejection detection pin
    pub type EjectionPin = Gpio24;

    /// UART RX
    pub type JupiterRxPin = Pin<Gpio16, FunctionUart, PullDown>;
    /// UART TX
    pub type JupiterTxPin = Pin<Gpio17, FunctionUart, PullDown>;

    /// I2C SDA pin
    pub type ThermoI2CSdaPin = Gpio32;
    /// I2C SCL pin
    pub type ThermoI2CSclPin = Gpio33;

    // /// GUARD SDA
    // pub type GuardSda = Pin<Gpio26, FunctionI2C, PullUp>;
    // /// GUARD SCL
    // pub type GuardScl = Pin<Gpio27, FunctionI2C, PullUp>;
}

use pins::*;
pub type ThermoI2cBus = 
    I2C<
        I2C0,
        (
            Pin<ThermoI2CSdaPin, FunctionI2C, PullUp>,
            Pin<ThermoI2CSclPin, FunctionI2C, PullUp>,
        ),
        Controller,
    >;

// SI1145
//pub type GuardI2C = I2C<I2C1, (GuardSda, GuardScl), Controller>;

// Heartbeat LED
pub type OnboardLED = Pin<OnboardLEDPin, FunctionSio<SioOutput>, PullNone>;

// Camera LED
pub type RedLed = Pin<RedLedPin, FunctionSio<SioOutput>, PullNone>;

// Camera LED
pub type GreenLed = Pin<GreenLedPin, FunctionSio<SioOutput>, PullNone>;

/// Ejection detection pin
pub type EjectionDetectionPin = Pin<EjectionPin, FunctionSio<SioInput>, PullDown>;

// JUPITER Uart
pub type JupiterUart = UartPeripheral<Enabled, UART0, (JupiterRxPin, JupiterTxPin)>;

/// Samples per second of the geiger counter
pub static SAMPLE_COUNT: usize = 100;
