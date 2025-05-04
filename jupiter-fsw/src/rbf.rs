use log::warn;

use crate::gpio::read::ReadPin;

pub struct RbfPin {
    pin: ReadPin,
    inserted_at_boot: bool,
}

impl From<ReadPin> for RbfPin {
    fn from(pin: ReadPin) -> Self {
        let boot = match pin.read() {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to read RBF pin: {:?}", e);
                true
            }
        };

        RbfPin {
            pin,
            inserted_at_boot: boot,
        }
    }
}

impl RbfPin {
    pub fn inhibited(&self) -> bool {
        // Inserted at boot
        self.inserted_at_boot
    }
}
