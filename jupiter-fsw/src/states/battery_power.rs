use bin_packets::phases::JupiterPhase;
use log::info;

use crate::states::shutdown::Shutdown;
use crate::timing::t_time_estimate;

use super::traits::{StateContext, ValidState};

// This the CONOPS say battery transition  in 3 seconds from TE2 warning,
static DELAY_TO_BATTERY_LATCH: i32 = 3;

#[derive(Debug, Clone, Default)]
pub struct BatteryPower {
    te_recieved_at: i32,
}

impl BatteryPower {
    pub fn enter() -> Self {
        Self {
            te_recieved_at: t_time_estimate(),
        }
    }

    fn te2_not_received(&self) -> bool {
        self.te_recieved_at <= 0
    }

    fn get_te2_status(&self, ctx: StateContext) -> Box<dyn ValidState> {
        if ctx.pins.te_2_high() {
            info!("Power off warning, activate battery latch in 3 seconds");
            Box::new(Self::enter())
        } else {
            Box::new(self.clone())
        }
    }
}

// This feels a little bit awkward without an intermediary state like "ejection"
// in between skirt seperation and battery latch activation
// but I didn't see a state like that in phases and didn't want to
// mess with the current state machine in main, so I went with this for now.
// Can be reworked later
impl ValidState for BatteryPower {
    fn phase(&self) -> JupiterPhase {
        JupiterPhase::BatteryPower
    }

    fn next(&self, ctx: StateContext) -> Box<dyn ValidState> {
        if self.te2_not_received() {
            self.get_te2_status(ctx)
        } else if self.te_recieved_at + DELAY_TO_BATTERY_LATCH < ctx.t_time {
            info!("Battery Latch Activated!");
            Box::new(Shutdown::enter())
        } else {
            Box::new(self.clone())
        }
    }
}
