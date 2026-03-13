use bincode::Decode;
use defmt::Format;
use serde::{Deserialize, Serialize};

use crate::{devices::DeviceIdentifier, packets::Encode};

#[derive(Copy, Clone, Encode, Decode, Format, Serialize, Deserialize, Debug)]
pub enum JupiterTestingPacket {
    SanityTest,
    SystemTest,
    OdinCommsTest,
    OdinStreamingTest,
    EjectorCommsTest,
}

#[derive(Copy, Clone, Encode, Decode, Format, Serialize, Deserialize, Debug)]
pub struct TestingPacket {
    system: DeviceIdentifier,
    test_type: TestType,
}

#[derive(Copy, Clone, Encode, Decode, Format, Serialize, Deserialize, Debug)]
pub struct TestingStatusPacket {
    system: DeviceIdentifier,
    test_type: TestType,
    status: TestStatus,
}

#[derive(Copy, Clone, Encode, Decode, Format, Serialize, Deserialize, Debug)]
pub enum TestType{
    SanityTest,
    SystemTest,
    RadioTest, 
    ThremocoupleTest,
    MotorSpinTest,
    PhotoDiodeTest,
    EjectorCommsTest,
    OdinCommsTest, 
    OdinStreamingTest,
    PowerLatchTest,
    EjectionTest,
    UartTest,
}

#[derive(Copy, Clone, Encode, Decode, Format, Serialize, Deserialize, Debug)]
pub enum OdinPiTestingPacket {
    SanityTest,
    RadioTest,
}

#[derive(Copy, Clone, Encode, Decode, Format, Serialize, Deserialize, Debug)]
pub enum OdinPicoTestingPacket {
    SanityTest,
    MotorSpinTest,
    PhotoDiodeTest,
}

#[derive(Copy, Clone, Encode, Decode, Format, Serialize, Deserialize, Debug)]
pub enum PowerPicoTestingPacket {
    SanityTest,
    ThremocoupleTest,
    PowerLatchTest,
}

#[derive(Copy, Clone, Encode, Decode, Format, Serialize, Deserialize, Debug)]
pub enum EjectorPicoTestingPacket {
    SanityTest,
    UartTest,
    EjectionTest,
}

#[derive(Copy, Clone, Encode, Decode, Format, Serialize, Deserialize, Debug)]
pub enum TelemetryPicoTestingPacket {
    SanityTest,
    UartTest,
    EjectionTest,
}

#[derive(Copy, Clone, Encode, Decode, Format, Serialize, Deserialize, Debug)]
pub enum TestStatus {
    Success, 
    Failure, 
    NotTested,
}