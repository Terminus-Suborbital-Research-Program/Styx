#![warn(missing_docs)]

use bin_packets::phases::JupiterPhase;
use embedded_hal::digital::PinState;

use crate::states::battery_power::BatteryPower;

use super::{
    skirt_seperation::SkirtSeperation,
    traits::{StateContext, ValidState},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct InfratrackerStart {}

impl InfratrackerStart {
    pub fn enter() -> Self {
        
    }
}

impl ValidState for InfratrackerStart {
    fn phase(&self) -> JupiterPhase {
        return JupiterPhase::Infratracking;
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        match ctx.atmega.pins().unwrap().te3() {
            PinState::High => {return Box::new(BatteryPower::default());},
            PinState::Low => {return Box::new(Self::default());},
        }
    }
}
