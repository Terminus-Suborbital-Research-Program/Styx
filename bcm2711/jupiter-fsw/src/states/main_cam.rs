use bin_packets::phases::JupiterPhase;
use embedded_hal::digital::PinState;

use super::{
    skirt_seperation::SkirtSeperation,
    traits::{StateContext, ValidState},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct Launch {}

impl ValidState for Launch {
    fn phase(&self) -> JupiterPhase {
        JupiterPhase::MainCamStart
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        match ctx.atmega.pins().unwrap_or_default().te1() {
            PinState::High => Box::new(SkirtSeperation::enter()),
            PinState::Low => Box::new(Self::default()),
        }
    }
}

