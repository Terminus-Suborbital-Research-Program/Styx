use rustfft::{Fft, FftPlanner, num_complex::Complex};
use std::sync::Arc;

use crate::signal::signal_config::SignalConfig;
use crate::sdr::radio_config::BUFF_SIZE;

pub struct SpectrumAnalyzer {
    fft: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    down_size: usize,
}

impl SpectrumAnalyzer {
    // FFt relies on transformation matrices computed for a specific size
    // since we always want to process signal data of the same size, we can compute
    // this once
    pub fn new(down_size: usize, len: usize) -> Self {
        let mut planner = FftPlanner::new();
        Self {
            fft: planner.plan_fft_forward(len),
            scratch: vec![Complex::new(0.0, 0.0); len],
            down_size,
        }
    }

    // Power Spectrum Density -> Take time series signal, convert to an fft power spectrum that can be compared with chi - square
    // Note that this currently will scramble the time series vec so use only after original time series is logged
    // this way there is zero copy
    pub fn psd(&mut self, time_series: &mut [Complex<f32>; BUFF_SIZE]) -> Vec<f32> {
        self.fft
            .process_with_scratch(time_series, &mut self.scratch);
        let mut power_spectrum: Vec<f32> = time_series
            .iter()
            .map(|complex| complex.norm_sqr())
            .collect();

        // Used to norm by signal length but that may be worse for chi - square
        // because it allows signal loudness to affect fit, so normalizing by total energy for now instead
        let total_energy: f32 = power_spectrum.iter().sum();
        if total_energy > 0.0 {
            let inv_energy = 1.0 / total_energy;
            for bin in &mut power_spectrum {
                *bin *= inv_energy;
            }
        }
        power_spectrum
    }

    pub fn spectral_bin_avg(&mut self, power_spectrum: Vec<f32>) -> Vec<f32> {
        // Length of spectral bin average buffer, which is more reliable for capturing correlation
        // and easier to sliding window
        let binned_len = power_spectrum.len() / &self.down_size;
        let mut spectral_average = Vec::with_capacity(binned_len);

        // Convert a set of x samples into one average sample. E.g. 65k becomes 1024 with chunks of it averaged
        for chunk in power_spectrum.chunks_exact(self.down_size) {
            let sum: f32 = chunk.iter().sum();
            spectral_average.push(sum / self.down_size as f32);
        }
        spectral_average
    }
}
