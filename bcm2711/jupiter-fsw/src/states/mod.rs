use bin_packets::phases::JupiterPhase;
use traits::{StateContext, ValidState};

mod battery_power;
mod ejection;
mod main_cam;
mod power_on;
mod shutdown;
mod skirt_seperation;

pub mod traits;
pub use power_on::*;

use crate::{
    gpio::write::WritePin,
    tasks::{IndicatorsReader, RbfReader},
    timing::t_time_estimate,
};

/// State machine for JUPITER
pub struct JupiterStateMachine {
    state: Box<dyn ValidState>,
    context: StateContext,
}

impl JupiterStateMachine {
    /// Create a new state machine from a pin provider
    pub fn new(pins: IndicatorsReader, ej_pin: WritePin, rbf: RbfReader) -> Self {
        Self {
            state: Box::new(PowerOn::default()),
            context: StateContext::new(pins, ej_pin, rbf),
        }
    }

    /// Update the state machine
    pub fn update(&mut self) {
        self.context.t_time = t_time_estimate();
        self.state = self.state.next(&mut self.context);
    }

    /// Get the current phase
    pub fn phase(&self) -> JupiterPhase {
        self.state.phase()
    }
}
