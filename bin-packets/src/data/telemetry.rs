use bincode::{Decode, Encode};
use defmt::Format;
use serde::{Deserialize, Serialize};

use crate::time::Timestamp;

/// Telemetry information for JUPITER
#[derive(Debug, Clone, Copy, Encode, Decode, Format, Serialize, Deserialize)]
pub struct JupiterTelemetry {
    pub battery_voltage: f32,
    pub timestamp: Timestamp,
    pub packet_number: u16,
    pub high_g_accel: f32, // Placeholder
    pub low_g_accel: f32,  // Placeholder
    pub gyro: f32,         // Placeholder
    pub temp_c: f32,       // Placeholder
    pub pressure_bar: f32, // Placeholder
    pub humidity: f32,     // Placeholder
}