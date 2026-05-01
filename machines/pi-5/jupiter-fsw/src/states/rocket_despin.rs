
use bin_packets::phases::JupiterPhase;

use crate::{states::{ejection::Ejection, launch::Launch}, timing::{self, t_time_estimate}};

use super::traits::{StateContext, ValidState};

const DELAY_TO_EJECT_SEC: i32 = 3; 

#[derive(Debug, Default)]
pub struct RocketDespin {
    te3_recieved_at: i32,
}

impl RocketDespin {
    pub fn enter() -> Self {
        // Set internal clock to TE+110
        timing::calibrate_to(110);
        Self {
            te3_recieved_at: t_time_estimate(),
        }
    }
}

impl ValidState for RocketDespin {
    fn phase(&self) -> bin_packets::phases::JupiterPhase {
        return JupiterPhase::RocketDespin;
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        if self.te3_recieved_at + DELAY_TO_EJECT_SEC < ctx.t_time {
            info!("Ejection complete, idling.");
            // ctx.ejection_pin.write(true).unwrap();


            Box::new(Ejection::default())
        } else {
            info!(
                "Waiting for ejection to complete. Time recieved: {}, current time: {}",
                self.te3_recieved_at, ctx.t_time
            );
            Box::new(Self::default())
        }
    }
}