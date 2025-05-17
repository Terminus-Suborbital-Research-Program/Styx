use embedded_hal::digital::{ErrorType, InputPin};
use log::{error, info, warn};

use super::{Pin, PinError};
use std::io::Read;
use std::process::{Command, Stdio};

pub struct ReadPin {
    pin: String,
}

impl ReadPin {
    pub(super) fn new<T: Into<String>>(pin: T) -> Self {
        ReadPin { pin: pin.into() }
    }
}

impl From<Pin> for ReadPin {
    fn from(pin: Pin) -> Self {
        let mut cmd = Command::new("gpioget")
            .arg(format!("{}", pin.pin()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        cmd.wait().unwrap();
        Self::new(pin.pin())
    }
}

impl ReadPin {
    pub fn read(&self) -> Result<bool, super::PinError> {
        let output = Command::new("gpioget")
        .arg(&self.pin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output().map_err(|e| {
            warn!("Failed to read pin {}: {}", self.pin, e);
            PinError::IoError(e)
        })?;

        match output.status.success() {
            false => {
                let err = String::from_utf8_lossy(&output.stderr);
                error!("Failed to read pin {}: {}", self.pin, err);
                Err(PinError::ParseError(err.to_string()))
            }

            true => {
                let line = String::from_utf8_lossy(&output.stdout);
                if line.contains("inactive") {
                    info!("Pin {} is low", self.pin);
                    Ok(false)
                } else if line.contains("active") {
                    info!("Pin {} is high", self.pin);
                    Ok(true)
                } else {
                    let err = String::from_utf8_lossy(&output.stderr);
                    error!("Failed to parse pin state: {}", err);
                    Err(PinError::ParseError(err.to_string()))
                }
            }
        }
    }
}

impl ErrorType for ReadPin {
    type Error = super::PinError;
}

impl InputPin for ReadPin {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        self.read()
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        let value = self.read()?;
        Ok(!value)
    }
}
