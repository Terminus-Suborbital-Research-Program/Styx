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
    parallel_filtering_v2::{parallel_fir_filter, ParallelFIRConfig, StreamingFIRFilter},
    resampling::downsample,
};

pub struct Downsampler {
    filter_i: StreamingFIRFilter,
    filter_q: StreamingFIRFilter,
    decimation_factor: usize,
    i_buf: Vec<f64>,
    q_buf: Vec<f64>,
}
impl Downsampler {

    pub fn new(input_sample_rate: f32, decimation_factor: usize, target_cutoff_hz: f32) -> Self {
        let n_taps = 818;
        let input_nyquist = input_sample_rate / 2.0;
        let normalized_cutoff = (target_cutoff_hz / input_nyquist) as f64;
        let window = "blackman";
        
        let taps_i = firwin(n_taps, normalized_cutoff, window, true).unwrap();
        let taps_q = firwin(n_taps, normalized_cutoff, window, true).unwrap();

        Self {
            filter_i: StreamingFIRFilter::new(taps_i).unwrap(),
            filter_q: StreamingFIRFilter::new(taps_q).unwrap(),
            decimation_factor,
            i_buf: vec![0.0; READ_CHUNK_SIZE],
            q_buf: vec![0.0; READ_CHUNK_SIZE],
        }
    }

    pub fn downsample(&mut self, raw_samples: &[Complex<f32>]) -> Vec<Complex<f32>>{
        let sample_len = raw_samples.len();

        for i in 0..sample_len {
            self.i_buf[i] = raw_samples[i].re as f64;
            self.q_buf[i] = raw_samples[i].im as f64;
        }

        let i_filtered = self.filter_i.process_block(&self.i_buf[..sample_len]);
        let q_filtered = self.filter_q.process_block(&self.q_buf[..sample_len]);

        // Return downsampled i and q components zipped up into complex type for future manipulation
        // with Rust fft
        i_filtered.into_iter()
                .zip(q_filtered.into_iter())
                .step_by(self.decimation_factor)
                .map(|(i,q,)| Complex::new(i as f32, q as f32))
                .collect()
    }
}

impl Default for Downsampler {
    fn default() -> Self {
        // Ideal N-taps according to Fred Harris rule of thumb and Kaiser windowing formula
        let n_taps = 818;
        let cutoff = 45_000.0 / 1_500_000.0;
        let window  = "blackman";
        
        let taps_i = firwin(n_taps, cutoff, window, true).unwrap();
        let taps_q = firwin(n_taps, cutoff, window, true).unwrap();

        Self {
            filter_i: StreamingFIRFilter::new(taps_i).unwrap(),
            filter_q:  StreamingFIRFilter::new(taps_q).unwrap(),
            decimation_factor: 30,
            i_buf: vec![0.0; READ_CHUNK_SIZE],
            q_buf: vec![0.0; READ_CHUNK_SIZE],
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
