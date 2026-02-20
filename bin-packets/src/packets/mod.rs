pub mod status;
pub mod testing;

use bincode::{Decode, Encode};
use defmt::Format;

use serde::{Deserialize, Serialize};
use status::Status;
//use testing::JupiterTestingPacket;

use crate::{
    commands::CommandPacket,
    packets::testing::{
        EjectorPicoTestingPacket, JupiterTestingPacket, OdinPiTestingPacket, OdinPicoTestingPacket,
        PowerPicoTestingPacket,
    },
};

#[derive(Debug, Clone, Copy, Encode, Decode, Format, Serialize, Deserialize)]
pub enum ApplicationPacket {
    Command(CommandPacket),
    Status(Status),
    JupiterTestingPacket(JupiterTestingPacket),
    OdinPiTestingPacket(OdinPiTestingPacket),
    OdinPicoTestingPacket(OdinPicoTestingPacket),
    EjectorPicoTesting(EjectorPicoTestingPacket),
    PowerPicoTesting(PowerPicoTestingPacket),
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
        temperature: u32,
        pressure: u32,
        humidity: u16,
    },
    PhotoresistorData {
        timestamp: u64,
        vector: [u16; 8],
    },
}
