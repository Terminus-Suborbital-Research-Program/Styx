
use bin_packets::phases::JupiterPhase;

use crate::states::{ejection::Ejection, launch::Launch};

use super::traits::{StateContext, ValidState};

const D:i32 = 3; 

#[derive(Debug, Default)]
pub struct RocketDespin {
    te2_recieved_at: i32,
}

impl ValidState for RocketDespin {
    fn phase(&self) -> bin_packets::phases::JupiterPhase {
        return JupiterPhase::RocketDespin;
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        if true {
            return Box::new(Ejection::default());
        }
        else {
            return Box::new(Self::default());
        }
    }
}