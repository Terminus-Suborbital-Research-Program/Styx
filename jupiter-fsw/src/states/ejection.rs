use bin_packets::phases::JupiterPhase;
use embedded_hal::digital::PinState;
use log::warn;

use crate::states::battery_power::BatteryPower;

use super::traits::ValidState;

#[derive(Debug, Clone, Copy, Default)]
pub struct Ejection {}

impl ValidState for Ejection {
    fn phase(&self) -> bin_packets::phases::JupiterPhase {
        JupiterPhase::EjectDeployable
    }

    fn next(&self, ctx: super::traits::StateContext) -> Box<dyn ValidState> {
        match ctx.pins.read().te1() {
            PinState::High => {
                // Low power warning, go to battery power
                log::info!("Received LV shutoff signal, triggering battery power");
                warn!("ATMEGA latch not implemented yet");
                Box::new(BatteryPower::default())
            }

            PinState::Low => {
                // No change
                Box::new(Self {})
            }
        }
    }
}
