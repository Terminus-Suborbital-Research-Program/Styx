pub mod status;

use bincode::{Decode, Encode};
use defmt::Format;

use serde::{Deserialize, Serialize};
use status::Status;

use core::option::Option;

use crate::commands::CommandPacket;

#[derive(Debug, Clone, Copy, Encode, Decode, Format, Serialize, Deserialize)]
pub enum ApplicationPacket {
    Command(CommandPacket),
    Status(Status),
    VoltageData {
        timestamp: u64,
        voltage: [f32; 3],
    },
    PowerData {
        name: u16,
        time_stamp: u64,
        power: Option<u16>,
    },
    CurrentData {
        timestamp: u64,
        currrent: [f32; 3],
    },
}
