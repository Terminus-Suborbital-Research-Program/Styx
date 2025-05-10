use bin_packets::phases::JupiterPhase;
use embedded_hal::digital::PinState;
use log::{info, warn};

use crate::states::main_cam::MainCam;

use super::traits::{StateContext, ValidState};

static MAIN_CAM_T_TIME: i32 = -30;

#[derive(Debug, Clone, Copy, Default)]
pub struct PowerOn {}

impl ValidState for PowerOn {
    fn phase(&self) -> JupiterPhase {
        JupiterPhase::PowerOn
    }

    fn next(&self, ctx: StateContext) -> Box<dyn ValidState> {
        if ctx.pins.read().te1() == PinState::High {
            // Crap, we have late power on for some reason
            warn!("Late power on: TE1 is high. Emergency transition to MainCamStart");
            return Box::new(MainCam::default());
        }

        if ctx.t_time > MAIN_CAM_T_TIME {
            // Go to main cam start
            info!("Starting main cam");
            Box::new(MainCam::default())
        } else {
            // Stay in power on
            Box::new(PowerOn::default())
        }
    }
}
