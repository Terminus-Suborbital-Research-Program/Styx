use hc12_rs::{configuration::baudrates::B9600, ProgrammingPair, FU3, HC12};
use pins::{
    CamLEDPin, CameraPin, EjectionPin, HeartbeatPin, JupiterRxPin, JupiterTxPin, RBFLEDPin, RBFPin,
    RadioProgrammingPin, RadioRxPin, RadioTxPin,
};
use rp235x_hal::{
    gpio::{FunctionSio, Pin, PullDown, PullNone, SioInput, SioOutput},
    pac::{UART0, UART1},
    timer::CopyableTimer1,
    uart::{Enabled, UartPeripheral},
    Timer,
};

use common::rbf::ActiveHighRbf;

pub mod pins {
    use rp235x_hal::gpio::{
        bank0::{
            Gpio13, Gpio14, Gpio15, Gpio16, Gpio17, Gpio2, Gpio20, Gpio21, Gpio25, Gpio8, Gpio9,
        },
        FunctionSio, FunctionUart, Pin, PullDown, SioInput, SioOutput,
    };

    // Ejector Heartbeat Output
    pub type HeartbeatPin = Gpio25;

    // Camera Startup should be right but the heartbeat and Cam LED Pins might be wrong
    // (inconsistency in ejector pinout doc) ask Brooks later

    // Camera Startup Pin
    pub type CameraPin = Gpio14;

    // Camera LED Pin
    pub type CamLEDPin = Gpio13;

    // RBF LED PIN
    pub type RBFLEDPin = Gpio15;

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
}

// Heartbeat LED
pub type Heartbeat = Pin<HeartbeatPin, FunctionSio<SioOutput>, PullNone>;

// pub type EjectorRBF = ActiveLowRbf<RBFPin>;
pub type EjectorRBF = ActiveHighRbf<RBFPin>;

// Camera Startup
pub type Camera = Pin<CameraPin, FunctionSio<SioOutput>, PullNone>;

// Camera LED
pub type CamLED = Pin<CamLEDPin, FunctionSio<SioOutput>, PullNone>;

// Camera LED
pub type RBFLED = Pin<RBFLEDPin, FunctionSio<SioOutput>, PullNone>;

/// Ejection detection pin
pub type EjectionDetectionPin = Pin<EjectionPin, FunctionSio<SioInput>, PullDown>;

// JUPITER Uart
pub type JupiterUart = UartPeripheral<Enabled, UART0, (JupiterRxPin, JupiterTxPin)>;

/// Ejector RBF
/// Represents the active-high Remove Before Flight (RBF) input.
pub type EjectorRbf = ActiveHighRbf<RBFPin>;

/// Radio UART
pub type RadioUart = UartPeripheral<Enabled, UART1, (RadioRxPin, RadioTxPin)>;

/// Radio HC12
pub type EjectorHC12 =
    HC12<RadioUart, ProgrammingPair<RadioProgrammingPin, Timer<CopyableTimer1>>, FU3<B9600>, B9600>;

pub mod packets {
    use super::{EjectorHC12, JupiterUart};

    /// Packet interface for the radio UART
    pub type RadioInterface = bin_packets::device::Device<EjectorHC12, 256>;

    /// Packet interface for the downlink UART to JUPITER
    pub type JupiterInterface = bin_packets::device::Device<JupiterUart, 256>;
}
