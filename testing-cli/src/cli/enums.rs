#![warn(missing_docs, redundant_imports, redundant_semicolons)]

use bin_packets::packets::{ApplicationPacket, testing::*};
use bincode::{Decode, Encode};
use clap::ValueEnum;

#[derive(Encode, Decode, ValueEnum, Clone, Debug)]
#[value(rename_all = "kebab-case")]
pub enum ElaraTests {
    JupiterSystemTest,
    JuptiterSanityTest,
    JupiterOdinCommsTest,
    JupiterOdinStreamingTest,
    JupiterEjectorCommsTest,
    OdinPiSanityTest,
    OdinPiRadioTest,
    OdinPicoSanityTest,
    OdinPicoMotorSpinTest,
    OdinPicoPhotoDiodeTest,
    PowerPicoSanityTest,
    PowerPicoThremocoupleTest,
    PowerPicoPowerLatchTest,
    EjectorPicoSanityTest,
    EjectorUartTest,
    EjectionTest,
    TelemetryPicoSanityTest,
    TelemetryPicoI2CTest,
    TelemetryPicoSensorTest,
}

#[derive(ValueEnum, Clone, Debug)]
#[value(rename_all = "kebab-case")]
pub enum Protocol {
    Usb,
    Bluetooth,
    IP,
}
