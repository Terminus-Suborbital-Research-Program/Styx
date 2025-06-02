use bin_packets::phases::JupiterPhase;
use common::battery_state::BatteryState;
use log::{info, warn};

use crate::states::shutdown::Shutdown;

use super::traits::{StateContext, ValidState};

#[derive(Debug, Clone, Default)]
pub struct BatteryPower {}

// For this - we're going to pull low at estimated T+600 seconds always, it's not informed by a pin
// the latch is triggered on entering this state, not by this state
static POWEROFF_T_TIME_SECS: i32 = 600;

impl ValidState for BatteryPower {
    fn phase(&self) -> JupiterPhase {
        JupiterPhase::BatteryPower
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        if ctx.t_time > POWEROFF_T_TIME_SECS {
            info!("Powering off latch");
            ctx.atmega.deactivate_latch();
            Box::new(Shutdown::enter())
        } else {
            // No change
            Box::new(Self::default())
        }
    }
}
