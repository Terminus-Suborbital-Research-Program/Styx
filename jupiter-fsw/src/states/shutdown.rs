use bin_packets::phases::JupiterPhase;
use log::info;

use crate::timing::t_time_estimate;

use super::traits::{StateContext, ValidState};

static DELAY_TO_SHUTDOWN: i32 = 255;


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
        JupiterPhase::BatteryPower
    }

    fn next(&self, ctx: StateContext) -> Box<dyn ValidState> {
        if self.time_since_switch  + DELAY_TO_SHUTDOWN < ctx.t_time {
            info!("Shutting Down!");
            // Replace with actual shutdown behavior
            Box::new(Self::enter())
        } else {
            Box::new(self.clone())
        }
        
    }
}
