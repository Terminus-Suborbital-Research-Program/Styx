use bin_packets::phases::JupiterPhase;

use crate::{
    gpio::write::WritePin,
    tasks::{Atmega, RbfReader},
    timing::t_time_estimate,
};

pub struct StateContext {
    pub t_time: i32,
    pub ejection_pin: WritePin,
    pub rbf: RbfReader,
    pub atmega: Atmega,
}

impl StateContext {
    pub fn new(atmega: Atmega, ejection_pin: WritePin, rbf: RbfReader) -> Self {
        Self {
            t_time: t_time_estimate(),
            ejection_pin,
            rbf,
            atmega,
        }
    }
}

pub trait ValidState {
    /// Get the current state as a telemetry phase
    fn phase(&self) -> JupiterPhase;

    /// Transition to the next state
    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState>;
}
