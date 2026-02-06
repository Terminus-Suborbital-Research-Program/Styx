use bincode::{Decode, Encode};
use defmt::Format;
use serde::{Deserialize, Serialize};

use crate::{devices::DeviceIdentifier, packets::ApplicationPacket, time::Timestamp};

#[derive(Debug, Clone, Copy, Encode, Decode, Format, Serialize, Deserialize)]
pub struct Status {
    /// Origin device
    pub device: DeviceIdentifier,
    /// Current timestamp
    pub timestamp_ns: Timestamp,
    /// Packet number
    pub sequence_number: u16,
}

impl Status {
    /// Create a new status packet
    pub fn new(device: DeviceIdentifier, timestamp_ns: Timestamp, sequence_number: u16) -> Self {
        Self {
            device,
            timestamp_ns,
            sequence_number,
        }
    }
}

impl From<Status> for ApplicationPacket {
    fn from(status: Status) -> Self {
        Self::Status(status)
    }
}
