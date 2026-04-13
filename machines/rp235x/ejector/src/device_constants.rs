//! Device constants and type definitions for the Ejector

#![warn(missing_docs, clippy::unwrap_used)]

use pins::{EjectionPin, JupiterRxPin, JupiterTxPin, OnboardLEDPin};
use rp235x_hal::{
    gpio::{FunctionI2C, FunctionSio, Pin, PullDown, PullNone, PullUp, SioInput, SioOutput},
    i2c::{Controller, Peripheral},
    pac::{I2C0, I2C1, UART0, UART1},
    timer::CopyableTimer1,
    uart::{Enabled, Reader, UartPeripheral, Writer},
    Timer, I2C,
};

#[allow(dead_code)]
pub mod pins {
    use rp235x_hal::gpio::{
        bank0::{
            Gpio0, Gpio1, Gpio10, Gpio11, Gpio12, Gpio16, Gpio17, Gpio2, Gpio20, Gpio21, Gpio24,
            Gpio25, Gpio26, Gpio27, Gpio32, Gpio33, Gpio4, Gpio5, Gpio6, Gpio7, Gpio8, Gpio9,
        },
        FunctionI2C, FunctionSio, FunctionUart, Pin, PullDown, PullUp, SioInput, SioOutput,
    };

    /// Ejector Heartbeat Output
    pub type OnboardLEDPin = Gpio25;

    // Camera Startup should be right but the heartbeat and Cam LED Pins might be wrong
    // (inconsistency in ejector pinout doc) ask Brooks later

    pub type Cam1Pin = Gpio10;

    pub type Cam2Pin = Gpio11;

    /// Camera GPIO activation
    pub type CamMosfetPin = Pin<Gpio12, FunctionSio<SioOutput>, PullDown>;

    pub type RGBLedPin = Gpio24;

    /// RBF PIN
    pub type RBFPin = Pin<Gpio2, FunctionSio<SioInput>, PullDown>;

    /// Ejection detection pin
    pub type EjectionPin = Gpio8;

    /// UART RX
    pub type JupiterRxPin = Pin<Gpio1, FunctionUart, PullDown>;
    /// UART TX
    pub type JupiterTxPin = Pin<Gpio0, FunctionUart, PullDown>;

    /// I2C SDA pin
    pub type ThermoI2CSdaPin = Gpio32;
    /// I2C SCL pin
    pub type ThermoI2CSclPin = Gpio33;

    // /// GUARD SDA
    // pub type GuardSda = Pin<Gpio26, FunctionI2C, PullUp>;
    // /// GUARD SCL
    // pub type GuardScl = Pin<Gpio27, FunctionI2C, PullUp>;
}

pub use pins::*;
/// I2C bus for the thermocouple
pub type ThermoI2cBus = I2C<
    I2C0,
    (
        Pin<ThermoI2CSdaPin, FunctionI2C, PullUp>,
        Pin<ThermoI2CSclPin, FunctionI2C, PullUp>,
    ),
    Controller,
>;

// SI1145
//pub type GuardI2C = I2C<I2C1, (GuardSda, GuardScl), Controller>;

pub type SDCardPins = u8;

// Heartbeat LED
pub type OnboardLED = Pin<OnboardLEDPin, FunctionSio<SioOutput>, PullNone>;

/// Camera LED
// pub type RedLed = Pin<RedLedPin, FunctionSio<SioOutput>, PullNone>;
pub type Cam1 = Pin<Cam1Pin, FunctionSio<SioOutput>, PullNone>;
pub type Cam2 = Pin<Cam2Pin, FunctionSio<SioOutput>, PullNone>;

/// Camera LED
// pub type GreenLed = Pin<GreenLedPin, FunctionSio<SioOutput>, PullNone>;

pub type RGBLed = Pin<RGBLedPin, FunctionSio<SioOutput>, PullNone>;

/// Ejection detection pin
pub type EjectionDetectionPin = Pin<EjectionPin, FunctionSio<SioInput>, PullDown>;

/// JUPITER Uart
// pub type JupiterUart = UartPeripheral<Enabled, UART0, (JupiterRxPin, JupiterTxPin)>;

// pub type JupiterRX = Reader<UART0, (JupiterRxPin, JupiterTxPin)>;

// pub type JupiterTX = Writer<UART0, (JupiterRxPin, JupiterTxPin)>;

pub type JupiterUart = UartPeripheral<Enabled, UART0, (JupiterTxPin, JupiterRxPin)>;

// Update these as well to match the new JupiterUart tuple order
pub type JupiterRX = Reader<UART0, (JupiterTxPin, JupiterRxPin)>;
pub type JupiterTX = Writer<UART0, (JupiterTxPin, JupiterRxPin)>;

/// Samples per second of the geiger counter
pub static SAMPLE_COUNT: usize = 100;

use ws2812_rs::Color;

pub struct RGBStatus {
    pub RBF: Color,
    pub HaLow: Color,
    pub Esp: Color,
    pub Infratracker: Color,
    pub Guard: Color,
    pub Jupiter: Color,
    pub ElectroMagnet: Color,
    pub Servos: Color,
    pub Jupiter_Avionics_Health: Color,
    pub Ejector_Health: Color,
    pub Odin_Compute_Health: Color,
    pub Odin_Pico_Health: Color,
}

use bin_packets::rgbstatus::{RGBOptions, WireColor};

impl RGBStatus {
    // Convert recieved binpacket colors to actual color
    pub fn update_from_options(&mut self, options: RGBOptions) {
        if let Some(c) = options.RBF {
            self.RBF = c.into();
        }
        if let Some(c) = options.HaLow {
            self.HaLow = c.into();
        }
        if let Some(c) = options.Esp {
            self.Esp = c.into();
        }
        if let Some(c) = options.Infratracker {
            self.Infratracker = c.into();
        }
        if let Some(c) = options.Guard {
            self.Guard = c.into();
        }
        if let Some(c) = options.Jupiter {
            self.Jupiter = c.into();
        }
        if let Some(c) = options.ElectroMagnet {
            self.ElectroMagnet = c.into();
        }
        if let Some(c) = options.Servos {
            self.Servos = c.into();
        }
        if let Some(c) = options.Jupiter_Avionics_Health {
            self.Jupiter_Avionics_Health = c.into();
        }
        if let Some(c) = options.Ejector_Health {
            self.Ejector_Health = c.into();
        }
        if let Some(c) = options.Odin_Compute_Health {
            self.Odin_Compute_Health = c.into();
        }
        if let Some(c) = options.Odin_Pico_Health {
            self.Odin_Pico_Health = c.into();
        }
    }
}

impl Default for RGBStatus {
    fn default() -> Self {
        let red = Color::red();
        Self {
            RBF: red,
            HaLow: Color::purple(),
            Esp: Color::orange(),
            Infratracker: red,
            Guard: red,
            Jupiter: red,
            ElectroMagnet: red,
            Servos: red,
            Jupiter_Avionics_Health: red,
            Ejector_Health: red,
            Odin_Compute_Health: red,
            Odin_Pico_Health: red,
        }
    }
}
