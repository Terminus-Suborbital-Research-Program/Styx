use bin_packets::phases::JupiterPhase;
use common::battery_state::BatteryState;
use embedded_hal::digital::PinState;
use log::{info, warn};

use crate::states::main_cam::Launch;

use super::traits::{StateContext, ValidState};

#[derive(Debug, Clone, Copy, Default)]
pub struct PowerOn {}

impl ValidState for PowerOn {
    fn phase(&self) -> JupiterPhase {
        JupiterPhase::PowerOn
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        if ctx.atmega.pins().unwrap_or_default().te1() == PinState::High {
            // Crap, we have late power on for some reason
            warn!("Late power on: TE1 is high. Emergency transition to MainCamStart");
            return Box::new(Launch::default());
        }

        if ctx.t_time > 0 {
            info!("Launch!");
            Box::new(Launch::default())
        } else {
            // Stay in power on
            ctx.atmega.set_battery_latch(BatteryState::LatchOn).unwrap();
            Box::new(PowerOn::default())
        }
    }
}
