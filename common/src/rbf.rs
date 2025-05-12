use bincode::{Decode, Encode};
use embedded_hal::digital::InputPin;

/// An RBF State
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Encode, Decode)]
pub enum RbfState {
    /// The RBF is inserted and the system is inhibited
    Inhibited,
    /// The RBF is not inserted
    Uninhibited,
}

impl core::fmt::Display for RbfState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RbfState::Inhibited => write!(f, "RBF Inhibited"),
            RbfState::Uninhibited => write!(f, "RBF Uninhibited"),
        }
    }
}

impl Into<bool> for RbfState {
    fn into(self) -> bool {
        match self {
            RbfState::Inhibited => true,
            RbfState::Uninhibited => false,
        }
    }
}

impl From<bool> for RbfState {
    fn from(value: bool) -> Self {
        if value {
            RbfState::Inhibited
        } else {
            RbfState::Uninhibited
        }
    }
}

/// Remove-before fire indicators are used at several points on flight hardware to prevent undisirable behavior
/// during testing. On the Ejector and on ICARUS, this inhibits servo movements, while on JUPITER it prevents 
/// the main-camera from starting up.
pub trait RbfIndicator {
    /// Is the RBF currently inserted?
    fn is_inserted(&mut self) -> bool;

    /// Was the RBF inserted at initialization?
    fn inhibited_at_init(&mut self) -> bool;

    /// Get the inhibition state currently
    fn get_inhibition(&mut self) -> RbfState {
        if self.is_inserted() || self.inhibited_at_init() {
            RbfState::Inhibited
        } else {
            RbfState::Uninhibited
        }
    }
}

/// An active-high (high = inserted) embedded-hal compatible RBF indicator
pub struct ActiveHighRbf<T: InputPin> {
    pin: T,
    inhibited_at_init: bool,
}

impl<T: InputPin> ActiveHighRbf<T> {
    pub fn new(mut pin: T) -> Self {
        let inhibited_at_init = pin.is_high().unwrap_or(false);
        Self {
            pin,
            inhibited_at_init,
        }
    }
}

impl<T: InputPin> RbfIndicator for ActiveHighRbf<T> {
    fn is_inserted(&mut self) -> bool {
        self.pin.is_high().unwrap_or(false)
    }

    fn inhibited_at_init(&mut self) -> bool {
        self.inhibited_at_init
    }
}