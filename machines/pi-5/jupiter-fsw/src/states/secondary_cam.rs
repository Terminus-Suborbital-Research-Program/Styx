
use embedded_hal::digital::PinState;
use bin_packets::phases::JupiterPhase;

use crate::states::{launch::Launch, rocket_despin::RocketDespin};

use super::traits::{StateContext, ValidState};
use crate::tasks::hardware::BoardHardware;

use log::info;


#[derive(Debug, Default)]
pub struct StartCameraRecording {
    te2_recieved_at: i32,
}

impl ValidState for StartCameraRecording {
    fn phase(&self) -> bin_packets::phases::JupiterPhase {
        return JupiterPhase::CamStart;
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        match ctx.hardware.pins().unwrap_or_default().te2() {
            PinState::High => { 
                info!("Cam recording complete, entering despin");
                return Box::new(RocketDespin::enter()); 
            },
            PinState::Low => { return Box::new(Self::default()); },
        }
    }
}