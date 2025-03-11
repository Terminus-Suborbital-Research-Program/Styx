use bincode::{Decode, Encode};
use defmt::Format;

use crate::phases::EjectorPhase;

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, Format)]
pub enum CommandPacket {
    SyncTime(u32),
    Ping,
    EjectorPhaseSet(EjectorPhase),
}
