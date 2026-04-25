use bin_packets::phases::JupiterPhase;
use embedded_hal::digital::PinState;

use crate::states::{secondary_cam::StartCameraRecording,}; //skirt_seperation::SkirtSeperation};

use super::traits::{StateContext, ValidState};

#[derive(Debug, Clone, Copy, Default)]
pub struct Launch {}

impl ValidState for Launch {
    fn phase(&self) -> JupiterPhase {
        return JupiterPhase::Launch;
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        if ctx.t_time >= 60 {
            return Box::new(StartCameraRecording::default());
        } else {
            return Box::new(Self::default());
        }
    }
}