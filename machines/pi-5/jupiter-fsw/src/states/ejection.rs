#![warn(missing_docs)]

use bin_packets::phases::JupiterPhase;
use bin_packets::packets::ApplicationPacket;
use bin_packets::commands::CommandPacket;
use bin_packets::phases::EjectorPhase;
use embedded_hal::digital::PinState;
use bin_packets::device::Device;
use bin_packets::device::PacketWriter;

use crate::states::{battery_power::BatteryPower, infratracker::InfratrackerStart};

use super::traits::{StateContext, ValidState};
use log::{info, error};

#[derive(Debug, Clone, Copy, Default)]
pub struct Ejection {}

impl ValidState for Ejection {
    fn phase(&self) -> bin_packets::phases::JupiterPhase {
        JupiterPhase::EjectDeployable
    }

    fn next(&self, ctx: &mut StateContext) -> Box<dyn ValidState> {
        if ctx.t_time >= 83 {
            info!("Ejection time wait complete, entering infratracker");
            if let Err(e) = ctx.ejection_pin.write(true) {
                error!("Failed to assert ejection pin: {:?}", e);
            }
            let i = 1;
            if let Some(iface) = ctx.interface.borrow_mut().as_mut() {
                // for i in 1..=5 {
                    let cmd = ApplicationPacket::Command(
                        CommandPacket::EjectorPhaseSet(EjectorPhase::Ejection)
                    ); 
                    
                    match iface.write(cmd) {
                        Ok(_) => info!("Eject command {}/5 successfully sent to Ejector.", i),
                        Err(e) => error!("Failed to send Eject command {}/5 to Ejector over UART: {}", i, e),
                    }
                // }
            } else {
                error!("Cannot send Eject command: UART interface is unavailable.");
            }
            return Box::new(InfratrackerStart::enter());
        } else {
            return Box::new(Self::default());
        }        
    }
}
