use log::warn;

use super::Pin;
use std::process::{Command, Stdio};

pub struct WritePin {
    pin: u8,
}

impl From<Pin> for WritePin {
    fn from(pin: Pin) -> Self {
        let mut cmd = Command::new("pigs")
            .arg("m")
            .arg(format!("{}", pin.pin()))
            .arg("w")
            .spawn()
            .expect("Failed to spawn pigs command");

        cmd.wait().ok();
        WritePin { pin: pin.pin() }
    }
}

impl WritePin {
    pub fn write(&self, high: bool) -> Result<(), super::PinError> {
        let mut cmd = Command::new("pigs")
            .arg("w")
            .arg(format!("{}", self.pin))
            .arg(if high { "1" } else { "0" })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                warn!("Failed to spawn command: {}", e);
                super::PinError::IoError(e)
            })?;

        cmd.wait().map_err(|e| {
            warn!("Failed to wait for command: {}", e);
            super::PinError::IoError(e)
        })?;

        Ok(())
    }
}
