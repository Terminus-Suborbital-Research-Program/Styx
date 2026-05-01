#![warn(missing_docs)]

use bin_packets::phases::JupiterPhase;
use traits::{StateContext, ValidState};

mod battery_power;
mod ejection;
mod main_cam;
mod power_on;
mod shutdown;
//mod skirt_seperation;
mod rocket_despin;
mod secondary_cam;
mod launch;
mod infratracker;

pub mod traits;
pub use power_on::*;

use crate::{
    gpio::write::WritePin,
    tasks::Atmega,
    timing::t_time_estimate,
};

use crate::tasks::{ActiveHardware, BoardHardware};
use bin_packets::{
    data::status, device::{PacketReader, PacketWriter, std::Device}, packets::ApplicationPacket
};

/// State machine for JUPITER
pub struct JupiterStateMachine {
    state: Box<dyn ValidState>,
    context: StateContext,
}

impl<D> JupiterStateMachine {
    /// Create a new state machine from a pin provider
    pub fn new(hardware: ActiveHardware, ejection_pin: WritePin, serial_interface: &mut Device<D>) -> Self {
        Self {
            state: Box::new(PowerOn::default()),
            context: StateContext::new(hardware, ejection_pin, serial_interface),
        }
    }

    /// Update the state machine
    pub fn update(&mut self) {
        self.context.t_time = t_time_estimate();
        self.state = self.state.next(&mut self.context);
    }

    /// Get the current phase
    pub fn phase(&self) -> JupiterPhase {
        self.state.phase()
    }
}
