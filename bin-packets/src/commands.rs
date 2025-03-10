use bincode::{Decode, Encode};
use defmt::Format;

use crate::{
    phases::{EjectorPhase, IcarusPhase, JupiterPhase},
    types::UnixTimestampMillis,
};

/// Commands a transition into a phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Format)]
pub enum DevicePhaseVariants {
    Jupiter(JupiterPhase),
    Icarus(IcarusPhase),
    Ejector(EjectorPhase),
}

/// Holds command variants
#[derive(Debug, Clone, Copy, Encode, Decode, Format)]
pub enum CommandVariants {
    SetTimeTo(UnixTimestampMillis),
    SetPhase(DevicePhaseVariants),
    Ping,
}
