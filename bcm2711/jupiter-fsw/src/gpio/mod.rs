use embedded_hal::digital::{Error, ErrorKind};

pub mod read;
pub mod write;

#[derive(Debug)]
#[allow(dead_code)]
pub enum PinError {
    IoError(std::io::Error),
    ParseError(String),
}

impl Error for PinError {
    fn kind(&self) -> ErrorKind {
        match self {
            PinError::IoError(_) => ErrorKind::Other,
            PinError::ParseError(_) => ErrorKind::Other,
        }
    }
}

#[derive(Debug)]
pub struct Pin {
    pin: String,
}

impl Pin {
    pub fn new(pin: &str) -> Self {
        Pin {
            pin: pin.to_string(),
        }
    }

    pub fn pin(&self) -> &str {
        &self.pin
    }
}
