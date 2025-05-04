pub mod read;
pub mod write;

#[derive(Debug)]
#[allow(dead_code)]
pub enum PinError {
    IoError(std::io::Error),
    ParseError(String),
}

#[derive(Debug)]
pub struct Pin {
    pin: u8,
}

impl Pin {
    pub fn new(pin: u8) -> Self {
        Pin { pin }
    }

    pub fn pin(&self) -> u8 {
        self.pin
    }
}
