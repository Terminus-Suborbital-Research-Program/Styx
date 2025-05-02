use embedded_hal_bus::i2c::AtomicDevice;
use mcf8316c_rs::controller::MotorController;
use pins::{AvionicsI2CSclPin, AvionicsI2CSdaPin, EscI2CSclPin, EscI2CSdaPin, LedPin};
use rp235x_hal::{
    async_utils::AsyncPeripheral,
    gpio::{FunctionI2C, FunctionSio, Pin, PullDown, PullNone, PullUp, SioInput, SioOutput},
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

    /// Flap servo PWM
    pub type RelayServoPWMPin = Gpio1;
    /// Flap servo PWM
    pub type FlapServoPWMPin = Gpio3;

    /// Flab servo mosfet
    pub type FlapMosfetPin = Gpio2;
    /// Relay servo mosfet
    pub type RelayMosfetPin = Gpio0;

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
use rp235x_hal::gpio::bank0::{Gpio12, Gpio8, Gpio9};
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
    ProgrammingPair<Pin<Gpio12, FunctionSio<SioOutput>, PullDown>, Timer<CopyableTimer1>>,
    FU3<B9600>,
    B9600,
>;

/// A motor controller on a shared bus
pub type ReactionWheelMotor = ();

// CONSTANTS FOR ALL
const HISTORY_BUFFER_LENGTH: usize = 10;

// Sensor Data Types
// use bin_packets::types::{PowerData, CurrentData, VoltageData};
#[derive(Debug, Default)]
pub struct INAData {
    pub p1_buffer: heapless::HistoryBuffer<bin_packets::ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub p2_buffer: heapless::HistoryBuffer<bin_packets::ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub p3_buffer: heapless::HistoryBuffer<bin_packets::ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub v1_buffer: heapless::HistoryBuffer<bin_packets::ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub v2_buffer: heapless::HistoryBuffer<bin_packets::ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub v3_buffer: heapless::HistoryBuffer<bin_packets::ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub i1_buffer: heapless::HistoryBuffer<bin_packets::ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub i2_buffer: heapless::HistoryBuffer<bin_packets::ApplicationPacket, HISTORY_BUFFER_LENGTH>,
    pub i3_buffer: heapless::HistoryBuffer<bin_packets::ApplicationPacket, HISTORY_BUFFER_LENGTH>,
}
