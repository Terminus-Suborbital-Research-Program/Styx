#![warn(missing_docs)]

use bincode::{Decode, Encode};
use defmt::Format;

use crate::phases::EjectorPhase;
use crate::rgbstatus::RGBOptions;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Encode, Decode,Format, Serialize, Deserialize)]
pub enum CommandPacket {
    SyncTime(u32),
    Ping,
    EjectorPhaseSet(EjectorPhase),
    ColorSet(RGBOptions),
}
