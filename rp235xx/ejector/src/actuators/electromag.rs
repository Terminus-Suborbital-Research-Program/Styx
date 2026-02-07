use embedded_hal::{digital::OutputPin, pwm::SetDutyCycle};
use rp235x_hal::pwm::{Channel, FreeRunning, Slice, A};

pub enum ElectroMagnetPolarity {
    State1,
    State2,
}

pub struct HBridge<C1, C2, P3>
where
    C1: SetDutyCycle,
    C2: SetDutyCycle,
    P3: OutputPin,
{
    input_pin1: C1,
    input_pin2: C2,
    sleep_pin: P3,
}

impl<C1: SetDutyCycle, C2: SetDutyCycle, P3: OutputPin> HBridge<C1, C2, P3> {
    pub fn new(_in_pin1: C1, _in_pin2: C2, _in_pin3: P3) -> Self {
        Self {
            input_pin1: _in_pin1,
            input_pin2: _in_pin2,
            sleep_pin: _in_pin3,
        }
    }

    pub fn sleep_pin_high(&mut self) -> () {
        self.sleep_pin.set_high().unwrap();
    }

    pub fn sleep_pin_low(&mut self) -> () {
        self.sleep_pin.set_low().unwrap();
    }
}

pub struct ElectroMagnet<C1, C2, P3>
where
    C1: SetDutyCycle,
    C2: SetDutyCycle,
    P3: OutputPin,
{
    duty_cycle_: u16,
    h_bridge: HBridge<C1, C2, P3>,
    polarity: ElectroMagnetPolarity,
}

impl<C1, C2, P3> ElectroMagnet<C1, C2, P3>
where
    C1: SetDutyCycle,
    C2: SetDutyCycle,
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
    pub fn polarity_switch(&mut self) -> () {
        match self.polarity {
            ElectroMagnetPolarity::State1 => {
                self.polarity = ElectroMagnetPolarity::State2;
                self.h_bridge.input_pin1.set_duty_cycle(self.duty_cycle_);
                self.h_bridge.input_pin2.set_duty_cycle(0);
            }
            ElectroMagnetPolarity::State2 => {
                self.polarity = ElectroMagnetPolarity::State1;
                self.h_bridge.input_pin1.set_duty_cycle(0);
                self.h_bridge.input_pin2.set_duty_cycle(self.duty_cycle_);
            }
        }
    }

    pub fn enable(&mut self) -> () {
        self.h_bridge.sleep_pin.set_low().unwrap();
    }

    pub fn disable(&mut self) -> () {
        self.h_bridge.sleep_pin.set_high().unwrap();
    }

    pub fn set_duty_cycle(&mut self, _duty_cycle: u16) -> () {
        self.duty_cycle_ = _duty_cycle;
        match self.polarity {
            ElectroMagnetPolarity::State1 => {
                self.h_bridge
                    .input_pin1
                    .set_duty_cycle(self.duty_cycle_)
                    .unwrap();
                self.h_bridge.input_pin2.set_duty_cycle(0).unwrap();
            }
            ElectroMagnetPolarity::State2 => {
                self.h_bridge.input_pin1.set_duty_cycle(0).unwrap();
                self.h_bridge
                    .input_pin2
                    .set_duty_cycle(self.duty_cycle_)
                    .unwrap();
            }
        }
    }
}
