#![warn(missing_docs)]

use embedded_hal::{digital::OutputPin, pwm::SetDutyCycle};
use rp235x_hal::{
    gpio,
    pwm::{Channel, FreeRunning, Slice, A},
};

/// Ejector servo types
pub type EjectionServoPin = gpio::bank0::Gpio0;
pub type EjectionServoPwm = rp235x_hal::pwm::Pwm0;
pub type EjectionServoSlice = Slice<EjectionServoPwm, FreeRunning>;
pub type EjectionServoMosfet =
    gpio::Pin<gpio::bank0::Gpio1, gpio::FunctionSioOutput, gpio::PullDown>;
pub type EjectionServo = Servo<
    Channel<EjectionServoSlice, A>,
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
static MAX_DUTY: u32 = 8200;
static MIN_DUTY: u32 = 2200;

pub static EJECTION_ANGLE: u16 = 145;
pub static HOLDING_ANGLE: u16 = 85;
// pub static LOCKING_SERVO_LOCKED: u16 = 105;
// pub static LOCKING_SERVO_UNLOCKED: u16 = 20;

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

// Ejector servo
pub struct EjectorServo {
    servo: Servo<
        Channel<EjectionServoSlice, A>,
        gpio::Pin<EjectionServoPin, gpio::FunctionPwm, gpio::PullDown>,
        EjectionServoMosfet,
    >,
}

impl EjectorServo {
    pub fn new(
        servo: Servo<
            Channel<EjectionServoSlice, A>,
            gpio::Pin<EjectionServoPin, gpio::FunctionPwm, gpio::PullDown>,
            EjectionServoMosfet,
        >,
    ) -> Self {
        Self { servo }
    }

    pub fn eject(&mut self) {
        self.servo.set_angle(EJECTION_ANGLE);
        self.servo.enable();
    }

    pub fn hold(&mut self) {
        self.servo.set_angle(HOLDING_ANGLE);
        self.servo.enable();
    }

    pub fn disable(&mut self) {
        self.servo.disable();
    }

    pub fn enable(&mut self) {
        self.servo.enable();
    }
}
