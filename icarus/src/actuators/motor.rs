use embedded_hal::pwm::SetDutyCycle;
use crate::actuators::PWM2a;

// Motors Configuration
pub type MotorXPWM = Motor<PWM2a, rp235x_hal::gpio::Pin<rp235x_hal::gpio::bank0::Gpio4, rp235x_hal::gpio::FunctionPwm, rp235x_hal::gpio::PullDown>>;

pub struct Motor<C, P> {
    channel: C,
    _pin: P, // Consume this pin please
}

impl<C, P> Motor<C, P> {
    pub fn new(channel: C, pin: P) -> Self {
        Self { channel, _pin: pin }
    }
}

impl<C, P> Motor<C, P>
where
    C: SetDutyCycle,
{
    pub fn set_speed(&mut self, speed_fraction: u8) {
        self.channel.set_duty_cycle_percent(speed_fraction);
    }
}