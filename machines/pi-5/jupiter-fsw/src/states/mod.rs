#![warn(missing_docs)]

use bin_packets::phases::JupiterPhase;
use traits::{StateContext, ValidState};

mod battery_power;
mod ejection;
mod main_cam;
mod power_on;
mod shutdown;
//mod skirt_seperation;
mod rocket_despin;
mod secondary_cam;
mod launch;
mod infratracker;

pub mod traits;
pub use power_on::*;

use crate::{
    gpio::write::WritePin,
    tasks::Atmega,
    timing::t_time_estimate,
};

use crate::tasks::{ActiveHardware, BoardHardware};


/// State machine for JUPITER
pub struct JupiterStateMachine {
    state: Box<dyn ValidState>,
    context: StateContext,
}

impl JupiterStateMachine {
    /// Create a new state machine from a pin provider
    pub fn new(atmega: ActiveHardware, ejection_pin: WritePin) -> Self {
        Self {
            state: Box::new(PowerOn::default()),
            context: StateContext::new(atmega, ejection_pin),
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
