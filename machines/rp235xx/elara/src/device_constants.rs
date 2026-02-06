use bin_packets::packets::ApplicationPacket;
use heapless::Deque;
use pins::{AvionicsI2CSclPin, AvionicsI2CSdaPin, EscI2CSclPin, EscI2CSdaPin};
use rp235x_hal::gpio::SioOutput;
use rp235x_hal::{
    gpio::{FunctionI2C, FunctionSio, Pin, PullDown, PullUp},
    i2c::Controller,
    pac::{I2C0, I2C1},
    I2C,
};

use crate::{hal::timer::CopyableTimer1, peripherals::async_i2c::AsyncI2c};

// State Machine

pub mod pins {
    use rp235x_hal::gpio::bank0::*;

    /// Flab servo mosfet
    pub type FlapMosfetPin = Gpio2;
    /// Relay servo mosfet
    pub type RelayMosfetPin = Gpio0;

    /// Flap servo PWM
    pub type FlapServoPWMGpio = Gpio3;
    /// Flap servo PWM
    pub type RelayServoPWMGpio = Gpio1;

    /// I2C SDA pin
    pub type AvionicsI2CSdaPin = Gpio6;
    /// I2C SCL pin
    pub type AvionicsI2CSclPin = Gpio7;

    // Mux pins are 14, 13, 11, 10 for S0, S1, S2, S3

    /// Mux S0
    pub type MuxS0Pin = Gpio14;
    /// Mux S1
    pub type MuxS1Pin = Gpio13;
    /// Mux S2
    pub type MuxS2Pin = Gpio11;
    /// Mux S3
    pub type MuxS3Pin = Gpio10;
    /// Mux Disable
    pub type MuxEPin = Gpio12;

    // pub type ADCMux = CD74HC4067<Pin<MuxS0Pin, rp235x_hal::gpio::FunctionSio<rp235x_hal::gpio::SioOutput>, rp235x_hal::gpio::PullDown>, Pin<MuxS1Pin, rp235x_hal::gpio::FunctionSio<rp235x_hal::gpio::SioOutput>, rp235x_hal::gpio::PullDown>, Pin<MuxS2Pin, rp235x_hal::gpio::FunctionSio<rp235x_hal::gpio::SioOutput>, rp235x_hal::gpio::PullDown>, Pin<MuxS3Pin, rp235x_hal::gpio::FunctionSio<rp235x_hal::gpio::SioOutput>, rp235x_hal::gpio::PullDown>, Pin<MuxEPin, rp235x_hal::gpio::FunctionSio<rp235x_hal::gpio::SioOutput>, rp235x_hal::gpio::PullDown>>;

    /// ESC I2C SDA pin
    pub type EscI2CSdaPin = Gpio16;
    /// ESC I2C SCL pin
    pub type EscI2CSclPin = Gpio17;
}

/// Servo items
pub mod servos {
    use rp235x_hal::{
        gpio::{FunctionPwm, FunctionSio, Pin, PullDown, SioOutput},
        pwm::{Channel, FreeRunning, Pwm0, Pwm1, Slice, B},
    };

    use crate::actuators::servo::Servo;

    use super::pins::{FlapMosfetPin, FlapServoPWMGpio, RelayMosfetPin, RelayServoPWMGpio};

    /// Flap mosfet pin
    pub type FlapMosfet = Pin<FlapMosfetPin, FunctionSio<SioOutput>, PullDown>;
    /// Relay mosfet pin
    pub type RelayMosfet = Pin<RelayMosfetPin, FunctionSio<SioOutput>, PullDown>;

    /// Flap servo PWM pin
    pub type FlapServoPwmPin = Pin<FlapServoPWMGpio, FunctionPwm, PullDown>;
    /// Relay servo PWM pin
    pub type RelayServoPwmPin = Pin<RelayServoPWMGpio, FunctionPwm, PullDown>;

    /// Flap servo PWM
    pub type FlapServoPwm = Pwm1;
    /// Relay servo PWM
    pub type RelayServoPwm = Pwm0;

    /// Flap servo slice
    pub type FlapServoSlice = Slice<FlapServoPwm, FreeRunning>;
    /// Relay servo slice
    pub type RelayServoSlice = Slice<RelayServoPwm, FreeRunning>;

    /// Flap Servo
    pub type FlapServo = Servo<Channel<FlapServoSlice, B>, FlapServoPwmPin, FlapMosfet>;
    /// Relay Servo
    pub type RelayServo = Servo<Channel<RelayServoSlice, B>, RelayServoPwmPin, RelayMosfet>;
}

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
        I2C0,
        (
            Pin<EscI2CSdaPin, FunctionI2C, PullUp>,
            Pin<EscI2CSclPin, FunctionI2C, PullUp>,
        ),
        Controller,
    >,
>;

use hc12_rs::*;
use rp235x_hal::gpio::bank0::{Gpio5, Gpio8, Gpio9};
use rp235x_hal::gpio::FunctionUart;
use rp235x_hal::pac::UART1;
use rp235x_hal::uart::Enabled;
use rp235x_hal::uart::UartPeripheral;
use rp235x_hal::Timer;

use hc12_rs::configuration::baudrates::B9600;

pub type IcarusHC12 = HC12<
    UartPeripheral<
        Enabled,
        UART1,
        (
            Pin<Gpio8, FunctionUart, PullDown>,
            Pin<Gpio9, FunctionUart, PullDown>,
        ),
    >,
    ProgrammingPair<Pin<Gpio5, FunctionSio<SioOutput>, PullDown>, Timer<CopyableTimer1>>,
    FU3<B9600>,
    B9600,
>;

/// Data buffer for downsyncing ICARUS data
pub type DownlinkBuffer = Deque<ApplicationPacket, 64>;
