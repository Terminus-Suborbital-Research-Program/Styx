use bincode::{Decode, Encode};
use defmt::Format;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode, Format)]
pub enum DeviceIdentifier {
    Jupiter,
    Icarus,
    Ejector,
    Atmega,
    Relay,
    Debug,
    Broadcast,
}
