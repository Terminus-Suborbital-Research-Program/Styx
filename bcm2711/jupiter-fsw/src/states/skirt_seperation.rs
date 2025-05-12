use bin_packets::phases::JupiterPhase;
use log::info;

use crate::states::ejection::Ejection;
use crate::timing::t_time_estimate;

use super::traits::{StateContext, ValidState};

static DELAY_TO_EJECT_SEC: i32 = 1;

#[derive(Debug)]
pub struct SkirtSeperation {
    te_recieved_at: i32,
}

impl SkirtSeperation {
    pub fn enter() -> Self {
        Self {
            te_recieved_at: t_time_estimate(),
        }
    }

    fn clone(&self) -> Self {
        Self {
            te_recieved_at: self.te_recieved_at,
        }
    }
}

impl ValidState for SkirtSeperation {
    fn phase(&self) -> JupiterPhase {
        JupiterPhase::SkirtSeperation
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        if self.te_recieved_at + DELAY_TO_EJECT_SEC < ctx.t_time {
            info!("Ejection complete, idling.");
            ctx.ejection_pin.write(false).unwrap();
            Box::new(Ejection::default())
        } else {
            info!("Waiting for ejection to complete.");
            ctx.ejection_pin.write(true).unwrap();
            Box::new(self.clone())
        }
    }
}
