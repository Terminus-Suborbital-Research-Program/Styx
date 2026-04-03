#![warn(missing_docs)]

use bin_packets::phases::JupiterPhase;
use embedded_hal::digital::PinState;

use super::{
    skirt_seperation::SkirtSeperation,
    traits::{StateContext, ValidState},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct MainCam {}

impl ValidState for MainCam {
    fn phase(&self) -> JupiterPhase {
        todo!()
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        todo!()
    }
}
