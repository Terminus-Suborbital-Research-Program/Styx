//! Code for the Ejector's electromagnet.

#![warn(missing_docs, clippy::unwrap_used)]

use embedded_hal::{digital::OutputPin, pwm::SetDutyCycle};
use rp235x_hal::pwm::{Channel, FreeRunning, Slice, A};

/// Polarity states for the electromagnet
pub enum ElectroMagnetPolarity {
    Attract,
    Repel,
}

/// H-Bridge struct for controlling the electromagnet
/// This is currently designed for the:
pub struct HBridge<P1, P2, P3>
where
    P1: OutputPin,
    P2: OutputPin,
    P3: OutputPin,
{
    pub input_pin1: P1,
    pub input_pin2: P2,
    pub sleep_pin: P3,
}

impl<P1: OutputPin, P2: OutputPin, P3: OutputPin> HBridge<P1, P2, P3> {
    /// Create a new H-Bridge instance
    pub fn new(_in_pin1: P1, _in_pin2: P2, _in_pin3: P3) -> Self {
        Self {
            input_pin1: _in_pin1,
            input_pin2: _in_pin2,
            sleep_pin: _in_pin3,
        }
    }

    /// Set the sleep pin high
    pub fn sleep_pin_high(&mut self) -> () {
        self.sleep_pin.set_high().unwrap();
    }

    /// Set the sleep pin low
    pub fn sleep_pin_low(&mut self) -> () {
        self.sleep_pin.set_low().unwrap();
    }

    pub fn bridge_state_00(&mut self) -> () {
        self.input_pin1.set_low().expect("msg");
        self.input_pin2.set_low().expect("msg");
    }
    pub fn bridge_state_01(&mut self) -> () {
        self.input_pin1.set_low().expect("msg");
        self.input_pin2.set_high().expect("msg");
    }
    pub fn bridge_state_10(&mut self) -> () {
        self.input_pin1.set_high().expect("msg");
        self.input_pin2.set_low().expect("msg");
    }
    /// S
    pub fn bridge_state_11(&mut self) -> () {
        self.input_pin1.set_high().expect("msg");
        self.input_pin2.set_high().expect("msg");
    }
}

/// Struct for managing the electromagnet, which uses an H-Bridge for control
pub struct ElectroMagnet<P1, P2, P3>
where
    P1: OutputPin,
    P2: OutputPin,
    P3: OutputPin,
{
    duty_cycle_: u16,
    h_bridge: HBridge<P1, P2, P3>,
    polarity: ElectroMagnetPolarity,
}

impl<P1, P2, P3> ElectroMagnet<P1, P2, P3>
where
    P1: OutputPin,
    P2: OutputPin,
    P3: OutputPin,
{
    pub fn new(_hbridge: HBridge<P1, P2, P3>, _polarity: ElectroMagnetPolarity) -> Self {
        Self {
            duty_cycle_: 0,
            h_bridge: _hbridge,
            polarity: _polarity,
        }
    }

    // TODO: Make sure th electromag starts in attract mode
    /// Switch the polarity of the electromagnet. Allowing for both attraction and repulsion.
    pub fn polarity_switch(&mut self) -> () {
        match self.polarity {
            ElectroMagnetPolarity::Attract => {
                self.polarity = ElectroMagnetPolarity::Repel;
                self.h_bridge.bridge_state_01();
            }
            ElectroMagnetPolarity::Repel => {
                self.polarity = ElectroMagnetPolarity::Attract;
                self.h_bridge.bridge_state_10();
            }
        }
    }

    /// Enable the electromagnet
    pub fn enable(&mut self) -> () {
        self.h_bridge.sleep_pin.set_low().unwrap();
    }

    /// Disable the electromagnet
    pub fn disable(&mut self) -> () {
        self.h_bridge.sleep_pin.set_high().unwrap();
    }
}
