use rp235x_hal::gpio::{bank0::Gpio21, FunctionSio, Pin, PullUp, SioInput};

pub type ListenPin = Pin<Gpio21, FunctionSio<SioInput>, PullUp>;
