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

use crate::tasks::IndicatorsReader;

/// State machine for JUPITER
pub struct JupiterStateMachine {
    state: Box<dyn ValidState>,
    pins: IndicatorsReader,
}

impl JupiterStateMachine {
    /// Create a new state machine from a pin provider
    pub fn new(pins: IndicatorsReader) -> Self {
        Self {
            state: Box::new(PowerOn::default()),
            pins,
        }
    }

    /// Update the state machine
    pub fn update(&mut self) {
        self.state = self.state.next(self.pins.clone().into());
    }

    /// Get the current phase
    pub fn phase(&self) -> JupiterPhase {
        self.state.phase()
    }
}
