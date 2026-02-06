use bin_packets::phases::JupiterPhase;
use common_states::rbf::RbfState;
use log::{info, warn};

use crate::timing::t_time_estimate;
use crate::{states::ejection::Ejection, timing};

use super::traits::{StateContext, ValidState};

static DELAY_TO_EJECT_SEC: i32 = 1;

#[derive(Debug)]
pub struct SkirtSeperation {
    te_recieved_at: i32,
}

impl SkirtSeperation {
    pub fn enter() -> Self {
        // Set internal clock to TE+110
        timing::calibrate_to(110);
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
            match ctx.rbf.read() {
                RbfState::Uninhibited => {
                    info!("Ejected!");
                    ctx.ejection_pin.write(true).unwrap();
                }

                RbfState::Inhibited => {
                    warn!("RBF Inserted, Not ejecting");
                }
            }
            Box::new(Ejection::default())
        } else {
            info!(
                "Waiting for ejection to complete. Time recieved: {}, current time: {}",
                self.te_recieved_at, ctx.t_time
            );
            if ctx.rbf.read() == RbfState::Uninhibited {
                ctx.ejection_pin.write(true).unwrap();
            }
            Box::new(self.clone())
        }
    }
}
