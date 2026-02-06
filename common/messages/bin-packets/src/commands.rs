use bincode::{Decode, Encode};
use defmt::Format;

use crate::phases::EjectorPhase;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, Format, Serialize, Deserialize)]
pub enum CommandPacket {
    SyncTime(u32),
    Ping,
    EjectorPhaseSet(EjectorPhase),
}
