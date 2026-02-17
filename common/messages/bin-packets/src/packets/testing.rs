use bincode::Decode;
use defmt::Format;
use serde::{Deserialize, Serialize};

use crate::packets::Encode;

#[derive(Copy, Clone, Encode, Decode, Format, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "testing", derive(clap::ValueEnum))]
pub enum JupiterTestingPacket {
    SanityTest,
    SystemTest,
    OdinCommsTest, 
    OdinStreamingTest, 
    EjectorCommsTest, 
}

#[derive(Copy, Clone, Encode, Decode, Format, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "testing", derive(clap::ValueEnum))]
pub enum OdinPiTestingPacket {
    SanityTest,
    RadioTest,
}

#[derive(Copy, Clone, Encode, Decode, Format, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "testing", derive(clap::ValueEnum))]
pub enum OdinPicoTestingPacket {
    SanityTest,
    MotorSpinTest,
    PhotoDiodeTest,
}

#[derive(Copy, Clone, Encode, Decode, Format, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "testing", derive(clap::ValueEnum))]
pub enum PowerPicoTestingPacket {
    SanityTest,
    ThremocoupleTest,
    PowerLatchTest,
}

#[derive(Copy, Clone, Encode, Decode, Format, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "testing", derive(clap::ValueEnum))]
pub enum EjectorPicoTestingPacket {
    SanityTest,
    UartTest, 
    EjectionTest,
}
