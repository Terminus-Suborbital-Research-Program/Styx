use log::{info, warn};

use super::Pin;
use std::process::{Command, Stdio};

pub struct WritePin {
    pin: String,
}

impl WritePin {
    pub(super) fn new<T: Into<String>>(pin: T) -> Self {
        WritePin { pin: pin.into() }
    }
}

impl From<Pin> for WritePin {
    fn from(pin: Pin) -> Self {
        Self::new(pin.pin())
    }
}

impl WritePin {
    pub fn write(&self, high: bool) -> Result<(), super::PinError> {
        let command = format!(
            "gpioset {}={}",
            self.pin,
            if high { "active" } else { "inactive" }
        );
        info!("Executing command: {}", command);
        let mut cmd = Command::new(command)
            .spawn()
            .map_err(|e| {
                warn!("Failed to spawn command: {e}");
                super::PinError::IoError(e)
            })?;

        // Wait for 200ms
        std::thread::sleep(std::time::Duration::from_millis(1000));

        // Try and kill the child
        if let Err(e) = cmd.kill() {
            warn!("Failed to kill process (already dead?): {e}");
        }

        Ok(())
    }
}
