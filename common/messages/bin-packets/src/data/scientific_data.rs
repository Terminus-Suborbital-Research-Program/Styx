use bincode::{Decode, Encode};
use defmt::Format;

use crate::time::{DurationMillis, Timestamp};

/// Data packet for GUARD Geiger counter
#[derive(Debug, Clone, Copy, Encode, Decode, Format)]
pub struct GeigerData {
    pub counts: u32,
    pub over: DurationMillis,
    pub timestamp: Timestamp,
    pub packet_number: u16,
}

/// Data packet for Peltier power generation
#[derive(Debug, Clone, Copy, Encode, Decode, Format)]
pub struct PeltierData {
    pub power: f32,
    pub temp_cold_c: f32,
    pub timestamp: Timestamp,
    pub packet_number: u16,
}

/// Data packet for Solar panel power generation
#[derive(Debug, Clone, Copy, Encode, Decode, Format)]
pub struct SolarData {
    pub power: f32,
    pub timestamp: Timestamp,
    pub packet_number: u16,
}
