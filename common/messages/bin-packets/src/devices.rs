#![warn(missing_docs)]

use bincode::{Decode, Encode};
use defmt::Format;

use serde::{Deserialize, Serialize};

/// Device identifiers for message routing and telemetry
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Encode, Decode, Format)]
pub enum DeviceIdentifier {
    Jupiter,
    Icarus, // <-- Replace with Odin???
    Ejector,
    Atmega,
    Relay,
    Debug,
    Broadcast,
}
