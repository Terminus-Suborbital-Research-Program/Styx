#![warn(missing_docs)]

use bin_packets::phases::JupiterPhase;

use crate::{
    gpio::write::WritePin,
    tasks::{Atmega, RbfReader},
    timing::t_time_estimate,
};


use crate::tasks::hardware::{ActiveHardware, BoardHardware};

// Active hardware is an alias for atmega or gpios, whichever we are using
pub struct StateContext {
    pub t_time: i32,
    pub ejection_pin: WritePin,
    pub hardware: ActiveHardware,
}

impl StateContext {
    pub fn new(hardware: ActiveHardware, ejection_pin: WritePin) -> Self {
        Self {
            t_time: t_time_estimate(),
            ejection_pin,
            hardware,
        }
    }
}

/// A trait that represents a valid state machine state
pub trait ValidState {
    /// Get the current state as a telemetry phase
    fn phase(&self) -> JupiterPhase;

    /// Transition to the next state
    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState>;
}
