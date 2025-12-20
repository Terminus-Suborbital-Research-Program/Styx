use core::time;

use crate::sdr::radio_config::RadioConfig;
use soapysdr::{Device, Direction, RxStream};
use rustfft::{num_complex::Complex};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SDR {
    device: Device,
    stream: RxStream<Complex<f32>>,
    buffer: Vec<Complex<f32>>,
}

impl SDR {
    pub fn new(config: RadioConfig) -> Result<Self,String> {
        let device = Device::new("").map_err(|e| e.to_string())?;
        
        device.set_frequency(Direction::Rx, 0, config.frequency, "").map_err(|e| e.to_string())?;
        device.set_sample_rate(Direction::Rx, 0, config.sample_rate).map_err(|e| e.to_string())?;
        
        if let Some(gain) = config.gain {
            device.set_gain(Direction::Rx, 0, gain).map_err(|e| e.to_string())?;
        }

        let mut stream = device.rx_stream::<Complex<f32>>(&[0]).map_err(|e| e.to_string())?;
        stream.activate(None).map_err(|e| e.to_string())?;

        Ok(Self {
            device,
            stream,
            buffer: vec![Complex::new(0.0, 0.0); config.read_chunk_size],
        })
    }

    pub fn fill_buffer(&mut self, accumulator: &mut Vec<Complex<f32>>) -> Result<u128, String> {
        let mut time_stamp = None;

        while accumulator.len() < accumulator.capacity() {
            let len = self.stream.read(&mut [&mut self.buffer], 100_000).map_err(|e| e.to_string())?;

            if time_stamp.is_none() {
                time_stamp = Some(SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos());
            }

            accumulator.extend_from_slice(&self.buffer[..len]);
        }

        Ok(time_stamp.unwrap_or(0))
    }

}