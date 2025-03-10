use bincode::{Decode, Encode};
use defmt::Format;

use crate::types::UnixTimestampMillis;

/// Telemetry information for JUPITER
#[derive(Debug, Clone, Copy, Encode, Decode, Format)]
pub struct JupiterTelemetry {
    pub battery_voltage: f32,
    pub timestamp: UnixTimestampMillis,
    pub packet_number: u16,
    pub high_g_accel: f32, // Placeholder
    pub low_g_accel: f32,  // Placeholder
    pub gyro: f32,         // Placeholder
    pub temp_c: f32,       // Placeholder
    pub pressure_bar: f32, // Placeholder
    pub humidity: f32,     // Placeholder
}

/// Telemetry information for ICARUS
#[derive(Debug, Clone, Copy, Encode, Decode, Format)]
pub struct IcarusTelemetry {
    pub battery_voltage: f32,
    pub timestamp: UnixTimestampMillis,
    pub packet_number: u16,
    pub accel: f32,  // Placeholder
    pub gyro: f32,   // Placeholder
    pub mag: f32,    // Placeholder
    pub temp_c: f32, // Placeholder
}
