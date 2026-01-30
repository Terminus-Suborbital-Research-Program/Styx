use crate::time::{DurationMillis, Timestamp};
use aether::attitude::Quaternion;
use aether::reference_frame::{
    Body,
    ICRF,
};
use bincode::{Decode, Encode};
use serde::{Serialize, Deserialize};

// #[derive(Debug, Clone, Copy, Encode, Decode, Format, Serialize, Deserialize)]
#[derive(Debug, Clone, Copy, Encode, Decode)]
pub struct AttitudeMetrics {
    timestamp: Timestamp,
    quaternion: Quaternion<f32, ICRF<f32>,Body<f32>>,
    signal_match: f32,
}