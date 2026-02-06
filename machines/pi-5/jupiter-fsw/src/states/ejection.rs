use bin_packets::phases::JupiterPhase;
use embedded_hal::digital::PinState;

use crate::states::battery_power::BatteryPower;

use super::traits::{StateContext, ValidState};

#[derive(Debug, Clone, Copy, Default)]
pub struct Ejection {}

impl ValidState for Ejection {
    fn phase(&self) -> bin_packets::phases::JupiterPhase {
        JupiterPhase::EjectDeployable
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        match ctx.atmega.pins().unwrap_or_default().te2() {
            PinState::High => {
                // Low power warning, go to battery power
                log::info!("Received LV shutoff signal, triggering battery power");
                ctx.atmega.activate_latch();
                Box::new(BatteryPower::default())
            }

            PinState::Low => {
                // No change
                Box::new(Self {})
            }
        }
    }
}
