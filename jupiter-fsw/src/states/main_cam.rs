use bin_packets::phases::JupiterPhase;

use super::{
    skirt_seperation::SkirtSeperation,
    traits::{StateContext, ValidState},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct MainCam {}

impl ValidState for MainCam {
    fn phase(&self) -> JupiterPhase {
        JupiterPhase::MainCamStart
    }

    fn next(&self, ctx: StateContext) -> Box<dyn ValidState> {
        if ctx.pins.te_1_high() {
            Box::new(SkirtSeperation::enter())
        } else {
            Box::new(Self::default())
        }
    }
}
