//! Code for the Ejector's servo control

#![warn(missing_docs, clippy::unwrap_used)]

use embedded_hal::{digital::OutputPin, pwm::SetDutyCycle};
use rp235x_hal::{
    gpio,
    pwm::{Channel, FreeRunning, Slice, B},
};

/// Ejector servo types
pub type EjectionServoPin = gpio::bank0::Gpio7;
pub type EjectionServoPwm = rp235x_hal::pwm::Pwm3;
pub type EjectionServoSlice = Slice<EjectionServoPwm, FreeRunning>;
pub type EjectionServoMosfet =
    gpio::Pin<gpio::bank0::Gpio6, gpio::FunctionSioOutput, gpio::PullDown>;
pub type EjectionServo = Servo<
    Channel<EjectionServoSlice, B>,
    gpio::Pin<EjectionServoPin, gpio::FunctionPwm, gpio::PullDown>,
    EjectionServoMosfet,
>;
// // Locking servo on ejector TURN NEGATIVE TO UNLOCK
// pub type LockingServoPin = gpio::bank0::Gpio2; // Physical pin 6
// pub type LockingServoPwm = rp235x_hal::pwm::Pwm1;
// pub type LockingServoSlice = Slice<LockingServoPwm, FreeRunning>;
// pub type LockingServo = Servo<
//     Channel<LockingServoSlice, A>,
//     gpio::Pin<LockingServoPin, gpio::FunctionPwm, gpio::PullDown>,
//     LockingServoMosfet,
// >;
// pub type LockingServoMosfet =
//     gpio::Pin<gpio::bank0::Gpio3, gpio::FunctionSioOutput, gpio::PullDown>;

// For the servo
// static MAX_DUTY: u32 = 8200;
// static MIN_DUTY: u32 = 2200;

const PWM_DIV_INT: u8 = 64;
const PWM_TOP: u16 = 46_874;

const TOP: u16 = PWM_TOP + 1;
// 0.5ms is 2.5% of 20ms; 0 degrees in servo
const MIN_DUTY: u16 = (TOP as f64 * (2.5 / 100.)) as u16; 
// 1.5ms is 7.5% of 20ms; 90 degrees in servo
// const HALF_DUTY: u16 = (TOP as f64 * (7.5 / 100.)) as u16; 
// 2.4ms is 12% of 20ms; 180 degree in servo
const MAX_DUTY: u16 = (TOP as f64 * (12.5 / 100.)) as u16;

pub static EJECTION_ANGLE: u16 = 240;
pub static HOLDING_ANGLE: u16 = 150;
// pub static LOCKING_SERVO_LOCKED: u16 = 105;
// pub static LOCKING_SERVO_UNLOCKED: u16 = 20;

/// Generic servo struct
pub struct Servo<C, P, M: OutputPin> {
    channel: C,
    _pin: P, // Consume this pin please
    mosfet_pin: M,
}

#[allow(dead_code)]
impl<C, P, M> Servo<C, P, M>
where
    M: OutputPin,
{
    pub fn new(channel: C, pin: P, mosfet_pin: M) -> Self {
        Self {
            channel,
            _pin: pin,
            mosfet_pin,
        }
    }
}

#[allow(dead_code)]
impl<C, P, M> Servo<C, P, M>
where
    C: SetDutyCycle,
    M: OutputPin,
{
    pub fn set_angle(&mut self, angle: u16) {
        let duty = ((angle as f32 / 180.0) * (MAX_DUTY - MIN_DUTY) as f32 + MIN_DUTY as f32) as u16;
        self.channel.set_duty_cycle(duty).unwrap();
    }

    pub fn enable(&mut self) {
        self.mosfet_pin.set_high().unwrap();
    }

    pub fn disable(&mut self) {
        self.mosfet_pin.set_low().unwrap();
    }
}

/// Ejector servo
pub struct EjectorServo {
    pub servo: Servo<
        Channel<EjectionServoSlice, B>,
        gpio::Pin<EjectionServoPin, gpio::FunctionPwm, gpio::PullDown>,
        EjectionServoMosfet,
    >,
}

impl EjectorServo {
    /// Create a new ejector servo instance
    pub fn new(
        servo: Servo<
            Channel<EjectionServoSlice, B>,
            gpio::Pin<EjectionServoPin, gpio::FunctionPwm, gpio::PullDown>,
            EjectionServoMosfet,
        >,
    ) -> Self {
        Self { servo }
    }

    /// Sets the servo to the ejection angle
    pub fn eject(&mut self) {
        self.servo.set_angle(EJECTION_ANGLE);
        self.servo.enable();
    }

    /// Hold the servo at the holding angle
    pub fn hold(&mut self) {
        self.servo.set_angle(HOLDING_ANGLE);
        self.servo.enable();
    }

    /// Disable the servo by setting the mosfet pin low
    pub fn disable(&mut self) {
        self.servo.disable();
    }

    pub fn enable(&mut self) {
        self.servo.enable();
    }
}
