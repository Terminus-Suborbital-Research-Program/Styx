use log::warn;

use crate::gpio::{PinError, read::ReadPin};

/// RBF State
pub enum RBFInhibit {
    /// Strong inhibition, inserted at boot
    Strong,
    /// Weak, currently inserted but not at boot
    Weak,
    /// Not inserted at boot not currently present
    None,
}

/// RBF state could not be determined
#[derive(Debug)]
pub struct IndetirminateRbf<T>(pub T);

/// RBF Pin reader
pub struct RbfPin {
    pin: ReadPin,
    inserted_at_boot: bool,
}

impl From<ReadPin> for RbfPin {
    fn from(pin: ReadPin) -> Self {
        let boot = match pin.read() {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to read RBF pin: {e:?}");
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
    pub fn inhibition(&self) -> Result<RBFInhibit, IndetirminateRbf<PinError>> {
        let inserted = self.pin.read().map_err(IndetirminateRbf)?;

        if inserted && self.inserted_at_boot {
            Ok(RBFInhibit::Strong)
        } else if inserted && !self.inserted_at_boot {
            Ok(RBFInhibit::Weak)
        } else {
            Ok(RBFInhibit::None)
        }
    }
}
