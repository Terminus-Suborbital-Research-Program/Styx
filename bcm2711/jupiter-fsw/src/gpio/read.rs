use embedded_hal::digital::{ErrorType, InputPin};
use log::warn;

use super::Pin;
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
        let mut cmd = Command::new("gpioget")
            .arg(format!("{}", self.pin))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                warn!("Failed to spawn command: {e}");
                super::PinError::IoError(e)
            })?;

        let mut output = String::new();
        if let Some(ref mut stdout) = cmd.stdout {
            stdout.read_to_string(&mut output).map_err(|e| {
                warn!("Failed to read stdout: {e}");
                super::PinError::IoError(e)
            })?;
        }

        let _status = cmd.wait().map_err(|e| {
            warn!("Failed to wait for command: {e}");
            super::PinError::IoError(e)
        })?;

        // Parse to 0 or 1, otherwise error
        if output.contains("active") {
            Ok(true)
        } else if output.contains("inactive") {
            Ok(false)
        } else {
            Err(super::PinError::ParseError(output))
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
