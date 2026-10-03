#![warn(missing_docs)]

/*
Phases for JUPITER, ICARUS, Ejector, and ELARA.
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

/// Phases for ELARA
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Format, Serialize, Deserialize)]
pub enum ElaraPhase {
    PreEject,
    Eject,
    ZeroRate,
    CassAFind,
    CassALock,
    EarthOrient,
    PartyMode,
    PowerDown,
    Calibration,
    Reset,
}

/// Phases for ejected satellite attitude sequencing.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Format, Serialize, Deserialize)]
pub enum EjectedSatellitePhase {
    Despin,
    TargetOrient,
    PowerOpt,
    PartyMode,
    Reset,
}

impl EjectedSatellitePhase {
    /// Returns the next phase in the nominal ejected-satellite sequence.
    pub const fn next(self) -> Self {
        match self {
            Self::Despin => Self::TargetOrient,
            Self::TargetOrient => Self::PowerOpt,
            Self::PowerOpt => Self::PartyMode,
            Self::PartyMode => Self::Reset,
            Self::Reset => Self::Reset,
        }
    }
}

impl Default for EjectedSatellitePhase {
    fn default() -> Self {
        Self::Despin
    }
}
