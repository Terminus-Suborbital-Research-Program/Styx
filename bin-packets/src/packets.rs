use bincode::{Decode, Encode};
use defmt::Format;

use serde::{Deserialize, Serialize};

use crate::data::EjectorStatus;
use crate::data::IcarusStatus;
use crate::{CommandPacket, DeviceIdentifier, JupiterStatus};
use core::option::Option;

#[derive(Debug, Clone, Copy, Encode, Decode, Format, Serialize, Deserialize)]
pub enum ApplicationPacket {
    Command(CommandPacket),
    Heartbeat {
        device: DeviceIdentifier,
        timestamp: u64,
        sequence_number: u16,
    },
    IcarusStatus(IcarusStatus),
    EjectorStatus(EjectorStatus),
    JupiterStatus(JupiterStatus),
    PowerData {
        time_stamp: u64,
        power: Option<(u8, u32)>,
    },
    CurrentData {
        time_stamp: u64,
        power: Option<(i8, u32)>,
    },
    VoltageData {
        time_stamp: u64,
        power: Option<(u8, u32)>,
    },
}
