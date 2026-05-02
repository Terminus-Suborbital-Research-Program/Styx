#![warn(missing_docs)]

use bin_packets::phases::JupiterPhase;
use embedded_hal::digital::PinState;

use crate::states::{battery_power::BatteryPower, infratracker::InfratrackerStart};

use super::traits::{StateContext, ValidState};
use log::info;

#[derive(Debug, Clone, Copy, Default)]
pub struct Ejection {}

impl ValidState for Ejection {
    fn phase(&self) -> bin_packets::phases::JupiterPhase {
        JupiterPhase::EjectDeployable
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        if ctx.t_time >= 90 {
            info!("Ejection time wait complete, entering infratracker");
            return Box::new(InfratrackerStart::enter());
        } else {
            return Box::new(Self::default());
        }        
    }
}
