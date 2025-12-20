
use std::borrow::Cow;
use serde::{Serialize, Deserialize};
use rustfft::num_complex::Complex;

#[derive(Serialize, Deserialize, Debug)]
pub struct SdrPacket<'a> {
    timestamp: u128,
    sample_count: u32,
    data: Cow<'a, [Complex<f32>]>, 
}

impl <'a>SdrPacket<'a> {
    pub fn new(timestamp: u128, accumulator: &'a Vec<Complex<f32>>)  -> Self {
        SdrPacket {
            timestamp,
            sample_count: accumulator.len() as u32,
            data: Cow::Borrowed(&accumulator), 
        }
    }
}