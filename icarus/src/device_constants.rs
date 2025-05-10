use bin_packets::{device::PacketDevice, packets::ApplicationPacket};
use pins::{AvionicsI2CSclPin, AvionicsI2CSdaPin, EscI2CSclPin, EscI2CSdaPin, LedPin};
use rp235x_hal::{
    gpio::{bank0::Gpio10, FunctionI2C, FunctionSio, Pin, PullDown, PullNone, PullUp, SioOutput},
    i2c::Controller,
    pac::{I2C0, I2C1},
    I2C,
};

use crate::{peripherals::async_i2c::AsyncI2c, phases::StateMachine};

// State Machine
pub type IcarusStateMachine = StateMachine<10>;

pub mod pins {
    use rp235x_hal::gpio::bank0::*;

    /// RBF Inhibit pin
    pub type RBFPin = Gpio4;

    /// Flab servo mosfet
    pub type FlapMosfetPin = Gpio2;
    /// Relay servo mosfet
    pub type RelayMosfetPin = Gpio0;

    /// Flap servo PWM
    pub type FlapServoPWMGpio = Gpio3;
    /// Flap servo PWM
    pub type RelayServoPWMGpio = Gpio1;

    /// I2C SDA pin
    pub type AvionicsI2CSdaPin = Gpio16;
    /// I2C SCL pin
    pub type AvionicsI2CSclPin = Gpio17;

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

    /// ESC I2C SDA pin
    pub type EscI2CSdaPin = Gpio18;
    /// ESC I2C SCL pin
    pub type EscI2CSclPin = Gpio19;

    /// Software controlled LED
    pub type LedPin = Gpio27;
}

/// Servo items
pub mod servos {
    use rp235x_hal::{
        gpio::{FunctionPwm, FunctionSio, Pin, PullDown, SioOutput},
        pwm::{Channel, FreeRunning, Pwm0, Pwm1, Slice, A, B},
    };

    use crate::actuators::servo::Servo;

    use super::pins::{FlapMosfetPin, FlapServoPWMGpio, RelayMosfetPin, RelayServoPWMGpio};

    /// Flap mosfet pin
    pub type FlapMosfet = Pin<FlapMosfetPin, FunctionSio<SioOutput>, PullDown>;
    /// Relay mosfet pin
    pub type RelayMosfet = Pin<RelayMosfetPin, FunctionSio<SioOutput>, PullDown>;

    pub static PWM_DIV_INT: u8 = 64;
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

    /// Flap servo locked
    pub static FLAP_SERVO_LOCKED: u16 = 50;
    /// Flap servo unlocked
    pub static FLAP_SERVO_UNLOCKED: u16 = 70;

    /// Relay servo locked
    pub static RELAY_SERVO_LOCKED: u16 = 90;
    /// Relay servo unlocked
    pub static RELAY_SERVO_UNLOCKED: u16 = 90;
}

/// Software-controlled LED
pub type SoftwareLED = Pin<LedPin, FunctionSio<SioOutput>, PullNone>;

// Avionics I2C bus
pub type AvionicsI2cBus = AsyncI2c<
    I2C<
        I2C0,
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

use crate::hal::timer::CopyableTimer1;
use hc12_rs::configuration::baudrates::B9600;
use hc12_rs::ProgrammingPair;
use hc12_rs::FU3;
use hc12_rs::HC12;
use rp235x_hal::gpio::bank0::{Gpio8, Gpio9};
use rp235x_hal::gpio::FunctionUart;
use rp235x_hal::pac::UART1;
use rp235x_hal::uart::Enabled;
use rp235x_hal::uart::UartPeripheral;
use rp235x_hal::Timer;

pub type IcarusHC12 = HC12<
    UartPeripheral<
        Enabled,
        UART1,
        (
            Pin<Gpio8, FunctionUart, PullDown>,
            Pin<Gpio9, FunctionUart, PullDown>,
        ),
    >,
    ProgrammingPair<Pin<Gpio10, FunctionSio<SioOutput>, PullDown>, Timer<CopyableTimer1>>,
    FU3<B9600>,
    B9600,
>;

/// Icarus HC12 Packet interface
pub type IcarusRadio = PacketDevice<IcarusHC12, 256>;

/// A motor controller on a shared bus
pub type ReactionWheelMotor = ();

// CONSTANTS FOR ALL
const HISTORY_BUFFER_LENGTH: usize = 10;

// Sensor Data Types
// use bin_packets::types::{PowerData, CurrentData, VoltageData};
#[derive(Debug, Default)]
pub struct INAData {
    pub p1_buffer: heapless::HistoryBuffer<ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub p2_buffer: heapless::HistoryBuffer<ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub p3_buffer: heapless::HistoryBuffer<ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub v1_buffer: heapless::HistoryBuffer<ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub v2_buffer: heapless::HistoryBuffer<ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub v3_buffer: heapless::HistoryBuffer<ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub i1_buffer: heapless::HistoryBuffer<ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub i2_buffer: heapless::HistoryBuffer<ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub i3_buffer: heapless::HistoryBuffer<ApplicationPacket, HISTORY_BUFFER_LENGTH>,
}
