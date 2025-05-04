use crate::tasks::SharedPinStates;
use bin_packets::phases::JupiterPhase;
use traits::ValidState;

mod battery_power;
mod ejection;
mod main_cam;
mod power_on;
mod shutdown;
mod skirt_seperation;

pub mod traits;
pub use power_on::*;

/// State machine for JUPITER
pub struct JupiterStateMachine {
    state: Box<dyn ValidState>,
    pins: SharedPinStates,
}

impl JupiterStateMachine {
    /// Create a new state machine from a pin provider
    pub fn new(pins: SharedPinStates) -> Self {
        Self {
            state: Box::new(PowerOn::default()),
            pins,
        }
    }

    /// Update the state machine
    pub fn update(&mut self) {
        let p = self.pins.read();
        self.state = self.state.next(p.into());
    }

    /// Get the current phase
    pub fn phase(&self) -> JupiterPhase {
        self.state.phase()
    }
}
