use crate::time::Timestamp;

use bincode::{Decode, Encode};
// use serde::{Serialize, Deserialize};

// Currently quaternions and reference frames pull STD, so transport a raw quaternion vector.
// The component order is scalar-first: [w, i, j, k].
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
    /// Quaternion encoded as [w, i, j, k].
    pub quaternion: [f32; 4],
    pub signal_match: f32,
}
