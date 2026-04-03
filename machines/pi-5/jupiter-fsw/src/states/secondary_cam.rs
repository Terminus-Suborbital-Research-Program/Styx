
use embedded_hal::digital::PinState;
use bin_packets::phases::JupiterPhase;

use crate::states::{launch::Launch, rocket_despin::RocketDespin};

use super::traits::{StateContext, ValidState};

#[derive(Debug, Default)]
pub struct StartCameraRecording {
    te2_recieved_at: i32,
}

impl ValidState for StartCameraRecording {
    fn phase(&self) -> bin_packets::phases::JupiterPhase {
        return JupiterPhase::CamStart;
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        match ctx.atmega.pins().unwrap_or_default().te2() {
            PinState::High => { return Box::new(RocketDespin::default()); },
            PinState::Low => { return Box::new(Self::default()); },
        }
    }
}