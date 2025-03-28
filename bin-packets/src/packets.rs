use bincode::{Decode, Encode};
use defmt::Format;

use serde::{Deserialize, Serialize};

use crate::data::EjectorStatus;
use crate::data::IcarusStatus;
use crate::{CommandPacket, DeviceIdentifier, JupiterStatus};

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
}
