use pins::{CamLEDPin, CameraPin, EjectionPin, HeartbeatPin, JupiterRxPin, JupiterTxPin};
use rp235x_hal::{
    gpio::{FunctionSio, Pin, PullDown, PullNone, SioInput, SioOutput},
    pac::UART0,
    uart::{Enabled, UartPeripheral},
};

pub mod pins {
    use rp235x_hal::gpio::{
        bank0::{Gpio13, Gpio14, Gpio15, Gpio16, Gpio17, Gpio21, Gpio25},
        FunctionUart, Pin, PullDown,
    };

    // Ejector Heartbeat Output
    pub type HeartbeatPin = Gpio25;

    // Camera Startup should be right but the heartbeat and Cam LED Pins might be wrong
    // (inconsistency in ejector pinout doc) ask Brooks later

    // Camera Startup Pin
    pub type CameraPin = Gpio14;

    // Camera LED Pin
    pub type CamLEDPin = Gpio13;

    /// Ejection detection pin
    pub type EjectionPin = Gpio21;

    /// UART TX
    pub type JupiterTxPin = Pin<Gpio17, FunctionUart, PullDown>;
    /// UART RX
    pub type JupiterRxPin = Pin<Gpio16, FunctionUart, PullDown>;
}

// Heartbeat LED
pub type Heartbeat = Pin<HeartbeatPin, FunctionSio<SioOutput>, PullNone>;

// Camera Startup
pub type Camera = Pin<CameraPin, FunctionSio<SioOutput>, PullNone>;

// Camera LED
pub type CamLED = Pin<CamLEDPin, FunctionSio<SioOutput>, PullNone>;

/// Ejection detection pin
pub type EjectionDetectionPin = Pin<EjectionPin, FunctionSio<SioInput>, PullDown>;

// JUPITER Uart
pub type JupiterUart = UartPeripheral<Enabled, UART0, (JupiterRxPin, JupiterTxPin)>;
