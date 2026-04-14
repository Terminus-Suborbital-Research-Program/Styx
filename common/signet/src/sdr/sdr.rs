use core::time;

use crate::sdr::radio_config::{
    RadioConfig,
    READ_CHUNK_SIZE,
    BUFF_SIZE,
    TARGET_PACKET_SIZE};
use bincode::de::read;
use rustfft::num_complex::Complex;
use soapysdr::{Device, Direction, RxStream};
use std::{any::Any, time::{SystemTime, UNIX_EPOCH}};
use crate::error::SignalError;
use scirs2_signal::{
    filter::firwin,
};

pub struct Downsampler {
    taps: Vec<f32>,
    history: Vec<Complex<f32>>,
    head: usize,
    decimation_factor: usize,
    skip_count: usize,
}

impl Downsampler {
    pub fn new(input_sample_rate: f32, decimation_factor: usize, target_cutoff_hz: f32) -> Self {
        let n_taps = 818;
        let input_nyquist = input_sample_rate / 2.0;
        let normalized_cutoff = (target_cutoff_hz / input_nyquist) as f64;
        let window = "blackman";
        
        // Use sci-rs to generate the perfect mathematical taps
        let taps_f64 = firwin(n_taps, normalized_cutoff, window, true).unwrap();
        
        let taps: Vec<f32> = taps_f64.into_iter().map(|t| t as f32).collect();

        Self {
            history: vec![Complex::new(0.0, 0.0); n_taps],
            taps,
            head: 0,
            decimation_factor,
            skip_count: 0, // Start computing immediately on the first sample
        }
    }

    /// A highly optimized Decimating FIR filter.
    /// It only computes the heavy dot-product for the samples it keeps.
    pub fn downsample(&mut self, raw_samples: &[Complex<f32>]) -> Vec<Complex<f32>> {
        let taps_len = self.taps.len();
        
        // Pre-allocate the exact size needed to avoid heap reallocation
        let expected_out_len = (raw_samples.len() + self.skip_count) / self.decimation_factor;
        let mut output = Vec::with_capacity(expected_out_len + 1);

        for &sample in raw_samples {
            // Write incoming sample to the ring buffer
            self.history[self.head] = sample;
            
            // Fast ring buffer advance (no modulo division)
            self.head += 1;
            if self.head >= taps_len {
                self.head = 0;
            }

            // Only compute the math if this is a sample we keep
            if self.skip_count == 0 {
                let mut sum_re = 0.0;
                let mut sum_im = 0.0;

                // Dot product (Iterating backwards through history)
                // We split the loop into two parts to avoid modulo division inside the hot loop.
                let mut tap_idx = 0;

                // Read from head down to 0
                for i in (0..self.head).rev() {
                    let val = self.history[i];
                    let tap = self.taps[tap_idx];
                    sum_re += val.re * tap;
                    sum_im += val.im * tap;
                    tap_idx += 1;
                }

                // Read from end of array down to head
                for i in (self.head..taps_len).rev() {
                    let val = self.history[i];
                    let tap = self.taps[tap_idx];
                    sum_re += val.re * tap;
                    sum_im += val.im * tap;
                    tap_idx += 1;
                }

                output.push(Complex::new(sum_re, sum_im));
                
                // Reset the skip counter
                self.skip_count = self.decimation_factor - 1;
            } else {
                // Skip the math for this sample
                self.skip_count -= 1;
            }
        }

        output
    }
}

impl Default for Downsampler {
    fn default() -> Self {
        // Ideal N-taps according to Fred Harris rule of thumb and Kaiser windowing formula
        let n_taps = 818;

        let cutoff = 45_000.0 / 1_500_000.0;
        let window  = "blackman";
        
        let taps_f64: Vec<f64> = firwin(n_taps, cutoff, window, true).unwrap();
        let taps: Vec<f32> = taps_f64.into_iter().map(|t| t as f32).collect();


       Self {
            history: vec![Complex::new(0.0, 0.0); n_taps],
            taps,
            head: 0,
            decimation_factor: 30,
            skip_count: 0, // Start computing immediately on the first sample
        }
    }
}


pub struct SDR {
    device: Device,
    pub stream: RxStream<Complex<f32>>,
    pub read_buffer: [Complex<f32>; READ_CHUNK_SIZE],
    downsampler: Downsampler,
}

use log::{error, info, LevelFilter};


impl SDR {
    pub fn new(config: RadioConfig) -> Result<Self, String> {
        let device = Device::new("biastee=true").map_err(|e| e.to_string())?;

        device
            .set_frequency(Direction::Rx, 0, config.frequency, "")
            .map_err(|e| e.to_string())?;
        device
            .set_sample_rate(Direction::Rx, 0, config.sample_rate)
            .map_err(|e| e.to_string())?;
        
        device
            .set_bandwidth(Direction::Rx, 0, config.sample_rate)
            .map_err(|e| e.to_string())?;

        // If manual gain is passed in - we're looking at the hydrogen line, otherwise we're using
        // Automatic gain control which is more helpful for radio stations.
        if let Some(gain_val) = config.gain {

            device
                .set_gain_mode(Direction::Rx, 0, false)
                .map_err(|e| format!("Failed to disable AGC: {}", e))?;
                
            device
                .set_gain(Direction::Rx, 0, gain_val)
                .map_err(|e| format!("Failed to set manual gain: {}", e))?;
        } else {
            device
                .set_gain_mode(Direction::Rx, 0, true)
                .map_err(|e| format!("Failed to enable AGC: {}", e))?;
        }

        let mut stream = device
            .rx_stream::<Complex<f32>>(&[0])
            .map_err(|e| e.to_string())?;
        stream.activate(None).map_err(|e| e.to_string())?;

        Ok(Self {
            device,
            stream,
            read_buffer: [Complex::new(0.0, 0.0); READ_CHUNK_SIZE],
            downsampler: Downsampler::default(),
        })
    }


    pub fn read_and_timestamp(&mut self, slice: &mut [Complex<f32>; BUFF_SIZE]) -> Result<(u128, usize), SignalError> {
        let mut time_stamp = None;
        let mut head: usize = 0;

        while head < TARGET_PACKET_SIZE{
            let read_len = self
                .stream
                .read(&mut [&mut self.read_buffer], 100_000)
                .map_err(|e| SignalError::StreamReadError(head))?;

            let downsampled_signal = self.downsampler.downsample(&self.read_buffer[..read_len]);
            let down_chunk_size = downsampled_signal.len();
            let end = head + down_chunk_size;

            
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
                slice[head..end].copy_from_slice(&downsampled_signal);
                head = end;
            } else {

                // May want to introduce overflow buffer of stuff that can't fit into packet size and store it in sdr
                // to be used in next call
                // Handle buffer overrun if a chunk pushes head past BUFF_SIZE
                let space_left = max - head;
                slice[head..max].copy_from_slice(&downsampled_signal[..space_left]);
                head = max;
                break;
            }

            // self.accumulator[prev_head..head] = self.read_buffer[..read_len];
            // accumulator.extend_from_slice(&self.buffer[..len]);
        }

        Ok((time_stamp.unwrap_or(0), head))
    }


}
