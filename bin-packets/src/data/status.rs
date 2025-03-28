use bincode::{Decode, Encode};
use defmt::Format;
use serde::{Deserialize, Serialize};

use crate::{
    phases::EjectorPhase,
    types::{DurationMillis, UnixTimestampMillis},
};

use super::JupiterTelemetry;

/// Status information for Ejector
#[derive(Debug, Clone, Copy, Encode, Decode, Format, Serialize, Deserialize)]
pub struct EjectorStatus {
    pub phase: EjectorPhase,
    pub time_in_phase: u64,
    pub timestamp: u64,
    pub packet_number: u16,
}

/// Status information for ICARUS
#[derive(Debug, Clone, Copy, Encode, Decode, Format, Serialize, Deserialize)]
pub struct IcarusStatus {
    pub time_in_phase: DurationMillis,
    pub timestamp: UnixTimestampMillis,
    pub packet_number: u16,
}

/// Status information for JUPITER
#[derive(Debug, Clone, Copy, Encode, Decode, Format, Serialize, Deserialize)]
pub struct JupiterStatus {
    pub time_in_phase: DurationMillis,
    pub timestamp: UnixTimestampMillis,
    pub packet_number: u16,
    pub telemetry: JupiterTelemetry,
}

/// Status packet for Relay
#[derive(Debug, Clone, Copy, Encode, Decode, Format)]
pub struct RelayStatus {
    pub timestamp: UnixTimestampMillis,
    pub packet_number: u16,
}
