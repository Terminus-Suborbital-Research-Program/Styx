/*
Phases for JUPITER, ICARUS, and Ejector. Other devices are
stateless and do not require any phase tracking.
*/

use bincode::{Decode, Encode};
use defmt::Format;

use serde::{Deserialize, Serialize};

/// Phases for JUPITER Pi
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Format, Serialize, Deserialize)]
pub enum JupiterPhase {
    PowerOn,
    MainCamStart,
    Launch,
    SkirtSeperation,
    EjectDeployable,
    BatteryPower,
    Shutdown,
}

/// Phases for ICARUS
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Format, Serialize, Deserialize)]
pub enum IcarusPhase {
    Ejection,
    FlapDeploy,
    OrientSolar,
    OrientReentry,
    FlapDeployment,
    Reentry,
}

/// Phases for Ejector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Format, Serialize, Deserialize)]
pub enum EjectorPhase {
    Standby,
    Ejection,
    Hold,
}
