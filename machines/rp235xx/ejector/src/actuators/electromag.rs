//! Code for the Ejector's electromagnet.

#![warn(missing_docs)]

use embedded_hal::{digital::OutputPin, pwm::SetDutyCycle};
use rp235x_hal::pwm::{Channel, FreeRunning, Slice, A};

/// Polarity states for the electromagnet
pub enum ElectroMagnetPolarity {
    Attract,
    Repel,
}

/// H-Bridge struct for controlling the electromagnet
/// This is currently designed for the:
pub struct HBridge<C1, C2, P3>
where
    C1: OutputPin,
    C2: OutputPin,
    P3: OutputPin,
{
    pub input_pin1: C1,
    pub input_pin2: C2,
    pub sleep_pin: P3,
}

impl<C1: OutputPin, C2: OutputPin, P3: OutputPin> HBridge<C1, C2, P3> {
    /// Create a new H-Bridge instance
    pub fn new(_in_pin1: C1, _in_pin2: C2, _in_pin3: P3) -> Self {
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
pub struct ElectroMagnet<C1, C2, P3>
where
    C1: OutputPin,
    C2: OutputPin,
    P3: OutputPin,
{
    duty_cycle_: u16,
    h_bridge: HBridge<C1, C2, P3>,
    polarity: ElectroMagnetPolarity,
}

impl<C1, C2, P3> ElectroMagnet<C1, C2, P3>
where
    C1: OutputPin,
    C2: OutputPin,
    P3: OutputPin,
{
    pub fn new(_hbridge: HBridge<C1, C2, P3>, _polarity: ElectroMagnetPolarity) -> Self {
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

    // Set the duty cycle for the electromagnet. This controls the strength of the magnetic field.
    //pub fn set_duty_cycle(&mut self, _duty_cycle: u16) -> () {
    //    self.duty_cycle_ = _duty_cycle;
    //    match self.polarity {
    //        ElectroMagnetPolarity::State1 => {
    //            self.h_bridge
    //                .input_pin1
    //                .set_duty_cycle(self.duty_cycle_)
    //                .unwrap();
    //            self.h_bridge.input_pin2.set_duty_cycle(0).unwrap();
    //        }
    //        ElectroMagnetPolarity::State2 => {
    //            self.h_bridge.input_pin1.set_duty_cycle(0).unwrap();
    //            self.h_bridge
    //                .input_pin2
    //                .set_duty_cycle(self.duty_cycle_)
    //                .unwrap();
    //        }
    //    }
    //}
}
