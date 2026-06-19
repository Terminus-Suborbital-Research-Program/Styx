
use bin_packets::phases::JupiterPhase;

use crate::{states::{ejection::Ejection, launch::Launch}, timing::{self, t_time_estimate}};

use super::traits::{StateContext, ValidState};


use log::info;


const DELAY_TO_EJECT_SEC:i32 = 3; 

#[derive(Debug, Default)]
pub struct RocketDespin {
    te2_recieved_at: i32,
}

impl RocketDespin {
    pub fn enter() -> Self {
        // Set internal clock to TE+110
        // 68 - skirt sep
        // 78 - skirt sep finish

        // TE 3 - 30 second to powerdown - t + 347
        timing::calibrate_to(78);
        Self {
            te2_recieved_at: t_time_estimate(),
        }
    }
}

impl ValidState for RocketDespin {
    fn phase(&self) -> bin_packets::phases::JupiterPhase {
        return JupiterPhase::RocketDespin;
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        if self.te2_recieved_at + DELAY_TO_EJECT_SEC < ctx.t_time {
            info!("Rocket despin complete, entering ejection.");
            Box::new(Ejection::default())
        } 
        else {
            return Box::new(Self::default());
        }
    }
}