use bin_packets::phases::JupiterPhase;

use crate::{tasks::IndicatorsReader, timing::t_time_estimate};

#[derive(Clone)]
pub struct StateContext {
    pub pins: IndicatorsReader,
    pub t_time: i32,
}

impl StateContext {
    pub fn new(pins: IndicatorsReader) -> Self {
        Self {
            pins,
            t_time: t_time_estimate(),
        }
    }
}

impl From<IndicatorsReader> for StateContext {
    fn from(pins: IndicatorsReader) -> Self {
        Self::new(pins)
    }
}

pub trait ValidState {
    /// Get the current state as a telemetry phase
    fn phase(&self) -> JupiterPhase;

    /// Transition to the next state
    fn next(&self, ctx: StateContext) -> Box<dyn ValidState>;
}
