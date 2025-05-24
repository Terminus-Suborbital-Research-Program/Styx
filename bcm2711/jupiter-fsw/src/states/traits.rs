use bin_packets::phases::JupiterPhase;

use crate::{
    gpio::write::WritePin,
    tasks::{IndicatorsReader, RbfReader, RbfTask},
    timing::t_time_estimate,
};

pub struct StateContext {
    pub pins: IndicatorsReader,
    pub t_time: i32,
    pub ejection_pin: WritePin,
    pub rbf: RbfReader,
}

impl StateContext {
    pub fn new(pins: IndicatorsReader, ejection_pin: WritePin, rbf: RbfReader) -> Self {
        Self {
            pins,
            t_time: t_time_estimate(),
            ejection_pin,
            rbf,
        }
    }
}

pub trait ValidState {
    /// Get the current state as a telemetry phase
    fn phase(&self) -> JupiterPhase;

    /// Transition to the next state
    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState>;
}
