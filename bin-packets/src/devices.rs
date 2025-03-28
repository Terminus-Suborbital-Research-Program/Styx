use bincode::{Decode, Encode};
use defmt::Format;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Encode, Decode, Format)]
pub enum DeviceIdentifier {
    Jupiter,
    Icarus,
    Ejector,
    Atmega,
    Relay,
    Debug,
    Broadcast,
}
