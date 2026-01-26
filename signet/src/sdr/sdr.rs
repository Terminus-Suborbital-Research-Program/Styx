use core::time;

use crate::sdr::radio_config::{
    RadioConfig,
    READ_CHUNK_SIZE,
    BUFF_SIZE,
    TARGET_PACKET_SIZE};
use bincode::de::read;
use rustfft::num_complex::Complex;
use soapysdr::{Device, Direction, RxStream};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::SignalError;
pub struct SDR {
    device: Device,
    stream: RxStream<Complex<f32>>,
    read_buffer: [Complex<f32>; READ_CHUNK_SIZE],
}

impl SDR {
    pub fn new(config: RadioConfig) -> Result<Self, String> {
        let device = Device::new("biastee=true").map_err(|e| e.to_string())?;

        device
            .set_frequency(Direction::Rx, 0, config.frequency, "")
            .map_err(|e| e.to_string())?;
        device
            .set_sample_rate(Direction::Rx, 0, config.sample_rate)
            .map_err(|e| e.to_string())?;

        if let Some(gain) = config.gain {
            device
                .set_gain(Direction::Rx, 0, gain)
                .map_err(|e| e.to_string())?;
        }

        let mut stream = device
            .rx_stream::<Complex<f32>>(&[0])
            .map_err(|e| e.to_string())?;
        stream.activate(None).map_err(|e| e.to_string())?;

        Ok(Self {
            device,
            stream,
            read_buffer: [Complex::new(0.0, 0.0); READ_CHUNK_SIZE],
        })
    }

    pub fn read_and_timestamp(&mut self, slice: &mut [Complex<f32>]) -> Result<(u128, usize), SignalError> {
        let mut time_stamp = None;
        let mut head: usize = 0;

        while head < TARGET_PACKET_SIZE{
            let read_len = self
                .stream
                .read(&mut [&mut self.read_buffer], 100_000)
                .map_err(|e| SignalError::StreamReadError(head))?;
            let end = head + read_len;

            if time_stamp.is_none() {
                time_stamp = Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos(),
                );
            }

            let max: usize = slice.len();
            if end <= max {
                slice[head..end].copy_from_slice(&self.read_buffer[..read_len]);
                head = end;
            } else {
                // Need to verify this is fine later on
                // I think the buffer logic works for collecting the parts of a read sample
                // before the one that exceeds the buffer bounds
                let final_section = max - head;
                slice[head..max].copy_from_slice(&self.read_buffer[..final_section]);
                break;
                // return Err(SignalError::PacketBufferOverflow(end));
            }

            // self.accumulator[prev_head..head] = self.read_buffer[..read_len];
            // accumulator.extend_from_slice(&self.buffer[..len]);
        }

        Ok((time_stamp.unwrap_or(0), head))
    }
}
