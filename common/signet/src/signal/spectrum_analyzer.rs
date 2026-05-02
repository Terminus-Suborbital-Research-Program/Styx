use rustfft::{Fft, FftPlanner, num_complex::Complex};
use std::sync::Arc;

use crate::sdr::radio_config::{BUFF_SIZE, TARGET_PACKET_SIZE};
use crate::signal::signal_config::SignalConfig;

const INTEGRATION_RATE: f32= 0.05;
pub struct SpectrumAnalyzer {
    fft: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    down_size: usize,
    pub integrated_psd: Option<Vec<f32>>, // Holds a rolling average for integation
    alpha: f32, // The integration rate 0.05 for a slow, smooth average
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
            integrated_psd: None,
            alpha: INTEGRATION_RATE
        }
    }

    // Power Spectrum Density -> Take time series signal, convert to an fft power spectrum that can be compared with chi - square
    // Note that this currently will scramble the time series vec so use only after original time series is logged
    // this way there is zero copy
    // Fine not being fixed size?
    pub fn psd(&mut self, time_series: &mut [Complex<f32>; BUFF_SIZE]) -> Vec<f32> {
        self.fft
            .process_with_scratch(&mut time_series[..TARGET_PACKET_SIZE], &mut self.scratch);
        let mut power_spectrum: Vec<f32> = time_series[..TARGET_PACKET_SIZE]
            .iter()
            .map(|complex| complex.norm_sqr())
            .collect();



        let mid = power_spectrum.len() / 2;
        power_spectrum.rotate_left(mid);
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
            let avg = sum / self.down_size as f32;

            // spectral_average.push(sum / self.down_size as f32);
            spectral_average.push(10.0 * (avg + 1e-12).log10());
        }
        if let Some(ref mut integrated) = self.integrated_psd {
            for (i, bin) in spectral_average.iter().enumerate() {
                // New Avg = (alpha * New Value) + ((1 - alpha) * Old Avg)
                integrated[i] = (self.alpha * bin) + ((1.0 - self.alpha) * integrated[i]);
            }
        } else {
            // First run, initialize the buffer
            self.integrated_psd = Some(spectral_average.clone());
        }

        self.integrated_psd.as_ref().unwrap().clone()
    }
}
