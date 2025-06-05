use pins::{
    EjectionPin, GreenLedPin, JupiterRxPin, JupiterTxPin, OnboardLEDPin, RadioProgrammingPin,
    RadioRxPin, RadioTxPin, RedLedPin,
};
use rp235x_hal::{
    gpio::{FunctionSio, Pin, PullDown, PullNone, SioInput, SioOutput},
    pac::{UART0, UART1},
    timer::CopyableTimer1,
    uart::{Enabled, UartPeripheral},
    Timer,
};

use common::rbf::NoRbf;

#[allow(dead_code)]
pub mod pins {
    use rp235x_hal::gpio::{
        bank0::{
            Gpio10, Gpio11, Gpio12, Gpio13, Gpio15, Gpio16, Gpio17, Gpio2, Gpio20, Gpio21, Gpio25,
            Gpio26, Gpio27, Gpio8, Gpio9,
        },
        FunctionI2C, FunctionSio, FunctionUart, Pin, PullDown, PullUp, SioInput, SioOutput,
    };

    // Ejector Heartbeat Output
    pub type OnboardLEDPin = Gpio25;

    // Camera Startup should be right but the heartbeat and Cam LED Pins might be wrong
    // (inconsistency in ejector pinout doc) ask Brooks later

    /// Camera GPIO activation
    pub type CamMosfetPin = Pin<Gpio12, FunctionSio<SioOutput>, PullDown>;

    // Camera LED Pin
    pub type RedLedPin = Gpio10;

    // RBF LED PIN
    pub type GreenLedPin = Gpio11;

    // RBF PIN
    pub type RBFPin = Pin<Gpio2, FunctionSio<SioInput>, PullDown>;

    /// Ejection detection pin
    pub type EjectionPin = Gpio21;

    /// UART RX
    pub type JupiterRxPin = Pin<Gpio16, FunctionUart, PullDown>;
    /// UART TX
    pub type JupiterTxPin = Pin<Gpio17, FunctionUart, PullDown>;

    /// Radio RX
    pub type RadioRxPin = Pin<Gpio8, FunctionUart, PullDown>;
    /// Radio TX
    pub type RadioTxPin = Pin<Gpio9, FunctionUart, PullDown>;
    /// Radio Programming Pin
    pub type RadioProgrammingPin = Pin<Gpio20, FunctionSio<SioOutput>, PullDown>;

    /// GUARD SDA
    pub type GuardSda = Pin<Gpio26, FunctionI2C, PullUp>;
    /// GUARD SCL
    pub type GuardScl = Pin<Gpio27, FunctionI2C, PullUp>;
}

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

/// Ejector RBF
/// Represents the active-high Remove Before Flight (RBF) input.
pub type EjectorRbf = NoRbf; //ActiveHighRbf<RBFPin>;

/// Radio UART
pub type RadioUart = UartPeripheral<Enabled, UART1, (RadioRxPin, RadioTxPin)>;

/// Radio HC12
pub type EjectorHC12 = UartPeripheral<Enabled, UART1, (RadioRxPin, RadioTxPin)>;

/// Samples per second of the geiger counter
pub static SAMPLE_COUNT: usize = 100;
