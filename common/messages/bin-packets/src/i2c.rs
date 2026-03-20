use bincode::{Decode, Encode};
use defmt::Format;
use serde::{Deserialize, Serialize};



#[derive(Debug, Clone, Copy, Encode, Decode, Format, Serialize, Deserialize)]
pub enum I2CPacket {
    PowerLatch = 0x002b,
}