use rustfft::num_complex::Complex;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use crate::sdr::radio_config::BUFF_SIZE;
use bytemuck::{Pod, Zeroable};

// May need to be paced to guarantee zero padding.
// packed
// #[repr(C)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct SdrPacketLog{
    pub timestamp: u128,
    pub sample_count: usize,
    #[serde(with = "serde_arrays")]
    pub samples: [Complex<f32>; BUFF_SIZE], 
}


impl SdrPacketLog {
    pub fn new(timestamp: u128, sample_count: usize, samples: [Complex<f32>; BUFF_SIZE], ) -> Self {
        Self {
            timestamp,
            sample_count,
            samples,
        }
    }
}

impl Default for SdrPacketLog {
    fn default() -> Self {
        Self {
            timestamp: 0,
            sample_count: 0,
            samples: [Complex::new(0.0, 0.0); BUFF_SIZE],
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct SdrPacketOwned {
    pub timestamp: u128,
    pub sample_count: usize,
    pub data: Vec<Complex<f32>>, 
}
