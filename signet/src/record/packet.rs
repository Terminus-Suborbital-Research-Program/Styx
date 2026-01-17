use rustfft::num_complex::Complex;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use crate::sdr::radio_config::BUFF_SIZE;

#[derive(Serialize, Deserialize, Debug)]
pub struct SdrPacket<'a> {
    timestamp: u128,
    sample_count: usize,
    data: Cow<'a, [Complex<f32>]>,
}

impl<'a> SdrPacket<'a> {
    pub fn new(timestamp: u128, accumulator: &'a [Complex<f32>], sample_count: usize) -> Self {
        SdrPacket {
            timestamp,
            sample_count,
            data: Cow::Borrowed(&accumulator[..sample_count]),
        }
    }
}
