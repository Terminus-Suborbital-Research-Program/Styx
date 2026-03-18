use crate::time::{Timestamp};


use bincode::{Decode, Encode};
// use serde::{Serialize, Deserialize};


// Currently quaternions and reference frames pull STD, so switching to tranposrting a raw vector
// #[cfg(feature = "aether")]
// use aether::attitude::Quaternion;

// #[cfg(feature = "aether")]
// use aether::reference_frame::{
//     Body,
//     ICRF,
// };

// #[derive(Debug, Clone, Copy, Encode, Decode, Format, Serialize, Deserialize)]
// #[cfg(feature = "aether")]
#[derive(Debug, Clone, Copy, Encode, Decode)]
pub struct AttitudeMetrics {
    pub timestamp: Timestamp,
    pub quaternion: [f32; 4],
    pub signal_match: f32,
}
