use bincode::{Decode, Encode};
use defmt::Format;

use crate::data::IcarusStatus;
use crate::DeviceIdentifier;
use crate::{data::EjectorStatus, phases::EjectorPhase};

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Format)]
pub struct ScientificPacket {
    pub packets: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, Format)]
pub enum CommandPacket {
    SyncTime(u32),
    Ping,
    ConnectionTest(ConnectionTest),
    EjectorPhaseSet(EjectorPhase),
}

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, Hash, Format)]
pub enum ConnectionTest {
    Start,
    Sequence(u8),
    End,
}

#[derive(Debug, Clone, Copy, Encode, Decode, Format)]
pub struct TelemetryPacket {
    pub gyro: (f32, f32, f32),
}

#[derive(Debug, Clone, Copy, Encode, Decode, Format)]
pub enum ApplicationPacket {
    Command(CommandPacket),
    Heartbeat {
        device: DeviceIdentifier,
        timestamp: u64,
        sequence_number: u16,
    },
    IcarusState(IcarusStatus),
    EjectorStatus(EjectorStatus),
}
