use crate::time::{Timestamp};


use bincode::{Decode, Encode};
use serde::{Serialize, Deserialize};

#[cfg(feature = "std")]
use aether::attitude::Quaternion;

#[cfg(feature = "std")]
use aether::reference_frame::{
    Body,
    ICRF,
};

// #[derive(Debug, Clone, Copy, Encode, Decode, Format, Serialize, Deserialize)]
#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy, Encode, Decode)]
pub struct AttitudeMetrics {
    pub timestamp: Timestamp,
    pub quaternion: Quaternion<f32, ICRF<f32>,Body<f32>>,
    pub signal_match: f32,
}
