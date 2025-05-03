use pins::{EjectionPin, JupiterRxPin, JupiterTxPin};
use rp235x_hal::{
    gpio::{FunctionSio, Pin, PullDown, SioInput},
    pac::UART0,
    uart::{Enabled, UartPeripheral},
};

pub mod pins {
    use rp235x_hal::gpio::{
        bank0::{Gpio16, Gpio17, Gpio21},
        FunctionUart, Pin, PullDown,
    };

    /// Ejection detection pin
    pub type EjectionPin = Gpio21;

    /// UART TX
    pub type JupiterTxPin = Pin<Gpio17, FunctionUart, PullDown>;
    /// UART RX
    pub type JupiterRxPin = Pin<Gpio16, FunctionUart, PullDown>;
}

/// Ejection detection pin
pub type EjectionDetectionPin = Pin<EjectionPin, FunctionSio<SioInput>, PullDown>;

// JUPITER Uart
pub type JupiterUart = UartPeripheral<Enabled, UART0, (JupiterRxPin, JupiterTxPin)>;
