pub mod status;

use bincode::{Decode, Encode};
use defmt::Format;

use serde::{Deserialize, Serialize};
use status::Status;

use crate::commands::CommandPacket;

#[derive(Debug, Clone, Copy, Encode, Decode, Format, Serialize, Deserialize)]
pub enum ApplicationPacket {
    Command(CommandPacket),
    Status(Status),
    VoltageData {
        timestamp: [u64; 4],
        voltage: [f32; 4],
    },
    PowerData {
        timestamp: [u64; 4],
        power: [f32; 4],
    },
    CurrentData {
        timestamp: [u64; 4],
        current: [f32; 4],
    },
    GeigerData {
        timestamp_ms: u64,
        recorded_pulses: u16,
    },
    JupiterAccelerometer {
        timestamp_ms: u64,
        vector: [f32; 3],
    },
}
