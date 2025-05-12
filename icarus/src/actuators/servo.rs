use embedded_hal::{digital::OutputPin, pwm::SetDutyCycle};

// Servo Consts
const PWM_TOP: u16 = 46_874;
const TOP: u16 = PWM_TOP + 1;
// 0.5ms is 2.5% of 20ms; 0 degrees in servo
const MIN_DUTY: u16 = (TOP as f64 * (2.5 / 100.)) as u16;
// 1.5ms is 7.5% of 20ms; 90 degrees in servo
const HALF_DUTY: u16 = (TOP as f64 * (7.5 / 100.)) as u16;
// 2.4ms is 12% of 20ms; 180 degree in servo
const MAX_DUTY: u16 = (TOP as f64 * (12. / 100.)) as u16;

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

    pub fn deg_0(&mut self){
        self.channel.set_duty_cycle(MIN_DUTY);
    }

    pub fn deg_90(&mut self){
        self.channel.set_duty_cycle(HALF_DUTY);
    }
    pub fn deg_180(&mut self){
        self.channel.set_duty_cycle(HALF_DUTY);
    }
    pub fn enable(&mut self) {
        self.mosfet_pin.set_high().unwrap();
    }

    pub fn disable(&mut self) {
        self.mosfet_pin.set_low().unwrap();
    }
}

pub struct ServoMultiMosfet<C, P, M1: OutputPin, M2: OutputPin> {
    channel: C,
    pin: P,
    mosfet1_pin: M1,
    mosfet2_pin: M2,
}

#[allow(dead_code)]
impl<C, P,  M1, M2> ServoMultiMosfet<C, P, M1, M2>
where
    M1: OutputPin,
    M2: OutputPin,
{
    pub fn new(channel: C, pin: P, mosfet1_pin: M1, mosfet2_pin: M2) -> Self {
        Self {
            channel,
            pin,
            mosfet1_pin,
            mosfet2_pin,
        }
    }
}

#[allow(dead_code)]
impl<C, P, M1, M2> ServoMultiMosfet<C, P, M1, M2>
where
    C: SetDutyCycle,
    M1: OutputPin,
    M2: OutputPin,{
    pub fn set_angle(&mut self, angle: u16) {
        let duty = ((angle as f32 / 180.0) * (MAX_DUTY - MIN_DUTY) as f32 + MIN_DUTY as f32) as u16;
        self.channel.set_duty_cycle(duty).unwrap();
    }
    pub fn deg_0(&mut self) {
        self.channel.set_duty_cycle(MIN_DUTY);
    }
    pub fn deg_90(&mut self) {
        self.channel.set_duty_cycle(HALF_DUTY);
    }
    pub fn deg_180(&mut self) {
        self.channel.set_duty_cycle(HALF_DUTY);
    }
    pub fn enable1(&mut self) {
        self.mosfet1_pin.set_high().unwrap();
    }
    pub fn enable2(&mut self) {
        self.mosfet2_pin.set_high().unwrap();
    }
    pub fn disable1(&mut self) {
        self.mosfet1_pin.set_low().unwrap();
    }
    pub fn disable2(&mut self) {
        self.mosfet2_pin.set_low().unwrap();
    }
}
