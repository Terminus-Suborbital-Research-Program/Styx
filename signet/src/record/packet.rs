use rustfft::num_complex::Complex;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use crate::sdr::radio_config::BUFF_SIZE;

#[derive(Serialize, Debug)]
pub struct SdrPacketLog<'a> {
    pub timestamp: u128,
    pub sample_count: usize,
    pub data: &'a [Complex<f32>], 
}

impl<'a> SdrPacketLog<'a> {
    pub fn new(timestamp: u128, sample_count: usize, data: &'a [Complex<f32>], ) -> Self {
        SdrPacketLog {
            timestamp,
            sample_count,
            data: &data[..sample_count],
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct SdrPacketOwned {
    pub timestamp: u128,
    pub sample_count: usize,
    pub data: Vec<Complex<f32>>, 
}
// #[derive(Serialize, Deserialize, Debug)]
// pub struct SdrPacket<'a> {
//     timestamp: u128,
//     sample_count: usize,
//     data: Cow<'a, [Complex<f32>]>,
// }

// impl<'a> SdrPacket<'a> {
//     pub fn new(timestamp: u128, accumulator: &'a [Complex<f32>], sample_count: usize) -> Self {
//         SdrPacket {
//             timestamp,
//             sample_count,
//             data: Cow::Borrowed(&accumulator[..sample_count]),
//         }
//     }
// }
