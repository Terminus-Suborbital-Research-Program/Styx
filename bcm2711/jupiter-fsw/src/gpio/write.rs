use log::warn;

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
        let mut cmd = Command::new("gpioset")
            .arg("-z") // Daemonize, we're going to kill it ourselves later
            .arg(format!("{}", self.pin))
            .arg(format!("{}={}", self.pin, if high { 1 } else { 0 }))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                warn!("Failed to spawn command: {e}");
                super::PinError::IoError(e)
            })?;

        // Wait for 200ms
        std::thread::sleep(std::time::Duration::from_millis(200));

        // Try and kill the child
        if let Err(e) = cmd.kill() {
            warn!("Failed to kill process (already dead?): {e}");
        }

        Ok(())
    }
}
