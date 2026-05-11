#![warn(missing_docs)]

use bin_packets::phases::JupiterPhase;
use log::info;

use crate::timing::t_time_estimate;

use super::traits::{StateContext, ValidState};
use std::process::Command;

use crate::tasks::BoardHardware;   

static DELAY_TO_SHUTDOWN: i32 = 30;

#[derive(Debug, Clone, Default)]
pub struct Shutdown {
    time_since_switch: i32,
}

// Not sure if this should just be totally reliant on t_time_estimate, or also
// rely on the time since battery latch release, so implementing this way for now
impl Shutdown {
    pub fn enter() -> Self {
        Self {
            time_since_switch: t_time_estimate(),
        }
    }
}

impl ValidState for Shutdown {
    fn phase(&self) -> JupiterPhase {
        JupiterPhase::Shutdown
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        if self.time_since_switch < ctx.t_time {
            info!("Syncing filesystem to prevent corruption...");
            let _ = Command::new("sync").status();

            // Second to finish writes
            std::thread::sleep(std::time::Duration::from_secs(1));

            info!("Shutting Down!");
            ctx.hardware.deactivate_latch();

            Box::new(Self::enter())
        } else {
            Box::new(self.clone())
        }
    }
}
