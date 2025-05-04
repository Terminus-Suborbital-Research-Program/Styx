use crate::gpio::read::ReadPin;

pub struct RbfPin {
    pin: ReadPin,
}

impl From<ReadPin> for RbfPin {
    fn from(pin: ReadPin) -> Self {
        RbfPin { pin }
    }
}

impl RbfPin {
    pub fn is_inserted(&self) -> bool {
        self.pin.read().unwrap()
    }
}
