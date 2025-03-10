use bincode::{Decode, Encode};
use defmt::Format;

use crate::{
    phases::EjectorPhase,
    types::{DurationMillis, UnixTimestampMillis},
};

/// Status information for Ejector
#[derive(Debug, Clone, Copy, Encode, Decode, Format)]
pub struct EjectorStatus {
    pub phase: EjectorPhase,
    pub time_in_phase: u64,
    pub timestamp: u64,
    pub packet_number: u16,
}

/// Status information for ICARUS
#[derive(Debug, Clone, Copy, Encode, Decode, Format)]
pub struct IcarusStatus {
    pub time_in_phase: DurationMillis,
    pub timestamp: UnixTimestampMillis,
    pub packet_number: u16,
}

/// Status information for JUPITER
#[derive(Debug, Clone, Copy, Encode, Decode, Format)]
pub struct JupiterStatus {
    pub time_in_phase: DurationMillis,
    pub timestamp: UnixTimestampMillis,
    pub packet_number: u16,
}

/// Status packet for Relay
#[derive(Debug, Clone, Copy, Encode, Decode, Format)]
pub struct RelayStatus {
    pub timestamp: UnixTimestampMillis,
    pub packet_number: u16,
}