use bin_packets::phases::JupiterPhase;

use crate::{tasks::PinStates, timing::t_time_estimate};

#[derive(Debug, Clone)]
pub struct StateContext {
    pub pins: PinStates,
    pub t_time: i32,
}

impl StateContext {
    pub fn new(pins: PinStates) -> Self {
        Self {
            pins,
            t_time: t_time_estimate(),
        }
    }
}

impl From<PinStates> for StateContext {
    fn from(pins: PinStates) -> Self {
        Self::new(pins)
    }
}

pub trait ValidState {
    /// Get the current state as a telemetry phase
    fn phase(&self) -> JupiterPhase;

    /// Transition to the next state
    fn next(&self, ctx: StateContext) -> Box<dyn ValidState>;
}
