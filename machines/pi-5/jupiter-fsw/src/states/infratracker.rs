#![warn(missing_docs)]

use bin_packets::phases::JupiterPhase;
use log::info;

use crate::states::{battery_power::BatteryPower, shutdown::Shutdown};

use super::{
    traits::{StateContext, ValidState},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct InfratrackerStart {}

static POWEROFF_T_TIME_SECS: i32 = 600;

impl ValidState for InfratrackerStart {
    fn phase(&self) -> JupiterPhase {
        return JupiterPhase::Infratracking;
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        if ctx.t_time > POWEROFF_T_TIME_SECS {
            info!("System Shutdown");
           // ctx.atmega.deactivate_latch();
            Box::new(Shutdown::enter())
        } else {
            // No change
            Box::new(Self::default())
        }
    }
}
