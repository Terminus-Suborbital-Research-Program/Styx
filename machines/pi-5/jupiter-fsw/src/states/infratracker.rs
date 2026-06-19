#![warn(missing_docs)]

use std::sync::atomic::Ordering;

use bin_packets::phases::JupiterPhase;
use log::info;

use crate::{states::{battery_power::BatteryPower, shutdown::Shutdown}, tasks::{TRACKING,BoardHardware}, timing::POWER_ON_TIME};

use super::{
    traits::{StateContext, ValidState},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct InfratrackerStart { 
    tracking: bool
}

impl InfratrackerStart {
    pub fn enter() -> Self {
        info!("Infratracker start - Begin tracking");
        TRACKING.store(true, Ordering::Relaxed);
        return  Self{tracking:  true};
    }
}
static POWEROFF_T_TIME_SECS: i32 = 600;
static INFRATRACKER_STOP: i32 = 342;


impl ValidState for InfratrackerStart {
    fn phase(&self) -> JupiterPhase {
        return JupiterPhase::Infratracking;
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        let mut next_tracking_state = self.tracking;

        if self.tracking && (ctx.t_time > INFRATRACKER_STOP) {
            info!("T+342s: InfraTracker Power Down");
            TRACKING.store(false, Ordering::Relaxed);
            next_tracking_state = false;
        }
        if ctx.t_time > POWEROFF_T_TIME_SECS {
            info!("System Shutdown");
           ctx.hardware.deactivate_latch();
            Box::new(Shutdown::enter())
        } else if ctx.hardware.pins().unwrap_or_default().te3().into() {
            // No change
            ctx.hardware.activate_latch();
            Box::new(BatteryPower::enter())
 
        } else {
            Box::new(Self { tracking: next_tracking_state })
        }
    }
}
