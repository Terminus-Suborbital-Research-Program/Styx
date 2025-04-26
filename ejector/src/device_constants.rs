use pins::EjectionPin;
use rp235x_hal::gpio::{FunctionSio, Pin, PullDown, SioInput};

pub mod pins {
    use rp235x_hal::gpio::bank0::Gpio21;

    /// Ejection detection pin
    pub type EjectionPin = Gpio21;
}

/// Ejection detection pin
pub type EjectionDetectionPin = Pin<EjectionPin, FunctionSio<SioInput>, PullDown>;
