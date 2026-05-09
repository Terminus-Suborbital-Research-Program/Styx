#![warn(missing_docs)]

pub mod status;

use bincode::{Decode, Encode};
use defmt::Format;

use serde::{Deserialize, Serialize};
use status::Status;

use crate::commands::CommandPacket;
use crate::i2c::I2CPacket;
// use crate::data::adcs::AttitudeMetrics;

#[derive(Debug, Clone, Copy, Encode, Decode, Format, Serialize, Deserialize)]
pub enum ApplicationPacket {
    Command(CommandPacket),
    Status(Status),
    I2C(I2CPacket),
    // ADCS(AttitudeMetrics),
    VoltageData {
        timestamp: [u64; 4],
        voltage: [f32; 4],
    },
    PowerData {
        timestamp: [u64; 4],
        power: [f32; 4],
    },
    CurrentData {
        timestamp: [u64; 4],
        current: [f32; 4],
    },
    GeigerData {
        timestamp_ms: u64,
        recorded_pulses: u16,
    },
    JupiterAccelerometer {
        timestamp_ms: u64,
        vector: [f32; 3],
    },
    AccelerometerData {
        timestamp: u64,
        x: f32,
        y: f32,
        z: f32,
    },
    MagnetometerData {
        timestamp: u64,
        x: f32,
        y: f32,
        z: f32,
    },
    GyroscopeData {
        timestamp: u64,
        x: f32,
        y: f32,
        z: f32,
    },
    EnvironmentData {
        timestamp: u64,
        temperature: f32,
        pressure: f32,
        humidity: f32,
    },
    BMPData {
        timestamp: u64,
        temperature: f32,
        pressure: f32,
    },
    BMEData {
        timestamp: u64,
        temperature: f32,
        pressure: f32,
        humidity: f32,
    },
    PhotoresistorData {
        timestamp: u64,
        vector: [u16; 8],
    },
    InfratrackerData {
        timestamp: u64,
        /// Quaternion encoded as [w, i, j, k].
        quaternion: [f32; 4],
    },
    ThermocoupleData {
        timestamp: u64,
        channel: u8,
        hot_junction_temp: f32,
    },
}
