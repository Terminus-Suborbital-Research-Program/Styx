use embedded_hal::{digital::OutputPin, pwm::SetDutyCycle};
use rp235x_hal::pwm::Channel;

pub enum ElectroMagnetPolarity {
    State1,
    State2,
}

pub struct HBridge<P1, P2, P3> 
where
    P1: OutputPin,
    P2: OutputPin,
    P3: OutputPin,
{
    input_pin1: P1,
    input_pin2: P2,
    sleep_pin : P3,
}

impl<P1, P2, P3> HBridge<P1, P2, P3> {
    pub fn new(_in_pin1: P1, _in_pin2: P2, _in_pin3: P3) -> Self {
        Self { input_pin1: _in_pin1, input_pin2: _in_pin2, sleep_pin: _in_pin3 }
    }

    pub fn pin1_high(&mut self) -> () {
        self.input_pin1.set_high().unwrap();
    }

    pub fn pin1_low(&mut self) -> () {
        self.input_pin1.set_low().unwrap();
    }

    pub fn pin2_high(&mut self) -> () {
        self.input_pin2.set_high().unwrap();
    }

    pub fn pin2_low(&mut self) -> () {
        self.input_pin2.set_low().unwrap();
    }

    pub fn sleep_pin_high(&mut self) -> () {
        self.sleep_pin.set_high().unwrap();
    }

    pub fn sleep_pin_low(&mut self) -> () {
        self.sleep_pin.set_low().unwrap();
    }
}

pub struct ElectroMagnet<C, P1, P2, P3> {
    channel: C,
    h_bridge: HBridge<P1, P2, P3>,
    polarity: ElectroMagnetPolarity,
}

impl<C, P1, P2, P3> ElectroMagnet<C, P1, P2, P3>
where
C: Channel,
    P1: OutputPin,
    P2: OutputPin,
    P3: OutputPin,
{
    pub fn new(channel: C, _h: HBridge<P1, P2, P3>, _polarity: ElectroMagnetPolarity) -> Self {
        Self {
            channel,
            h_bridge: _h,
            polarity: _polarity,
        }
    }

    pub fn polarity_switch(&mut self) -> () {
        match self.polarity {
            (ElectroMagnetPolarity::State1) => {
                self.polarity = ElectroMagnetPolarity::State2;
                self.h_bridge.pin1_low();
                self.h_bridge.pin2_high();
            }
            (ElectroMagnetPolarity::State2) => {
                self.polarity = ElectroMagnetPolarity::State1;
                self.h_bridge.pin1_high();
                self.h_bridge.pin2_low();
            }
        }
    }

    pub fn enable(&mut self) -> () {
        self.h_bridge.sleep_pin.set_low().unwrap();
    }

    pub fn disable(&mut self) -> () {
        self.h_bridge.sleep_pin.set_high().unwrap();
    }

    pub fn set_duty_cycle(&mut self, _duty_cycle: f32) -> () {
        self.channel.set_duty_cycle(_duty_cycle).unwrap();
    }

}