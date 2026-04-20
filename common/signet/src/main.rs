use rustfft::num_complex::Complex;
use signet::{
    record::{
        log::{SignalLogger, SignalReader},
        packet::SdrPacketLog,
    },
    sdr::{radio_config::{
        RadioConfig,
        TARGET_PACKET_SIZE,
        BUFF_SIZE,
        READ_CHUNK_SIZE
        
    }, sdr::{SDR, Downsampler}
    },
    signal::{estimator::MatchingEstimator, spectrum_analyzer::SpectrumAnalyzer},
    tools::cli::{Cli, Commands},
};

fn main() {
    let mut expected_average: Vec<f32> = Vec::new();

    let (radio_config, signal_config) = Cli::get_configs();
    let (record_baseline, psd_path) = Cli::run_commands();

    let mut sdr = SDR::new(radio_config).unwrap();
    let mut spectrum_analyzer = SpectrumAnalyzer::new(signal_config.down_size, TARGET_PACKET_SIZE);

    // let mut accumulator: Vec<Complex<f32>> = Vec::with_capacity(radio_config.target_packet_size + radio_config.read_chunk_size);

    let mut samples: [Complex<f32>; BUFF_SIZE] = [Complex::new(0.0, 0.0); BUFF_SIZE];
    // if record_baseline {
    //     let mut psd_recorder = SignalLogger::new(psd_path.to_str().unwrap());
    //     let (time_stamp, samples_read) = sdr.read_and_timestamp(&mut samples).unwrap();
    //     let power_spectrum = spectrum_analyzer.psd(&mut samples);
    //     let power_spectrum_bin_averaged = spectrum_analyzer.spectral_bin_avg(power_spectrum);

    //     psd_recorder.record_psd(power_spectrum_bin_averaged);
    //     return;
    // }
    if record_baseline {
        println!("Integrating baseline over 100 frames. Please wait...");
        let mut final_psd = Vec::new();
        
        for _ in 0..100 {
            let (_time_stamp, _samples_read) = sdr.read_and_timestamp(&mut samples).unwrap();
            let power_spectrum = spectrum_analyzer.psd(&mut samples);
            final_psd = spectrum_analyzer.spectral_bin_avg(power_spectrum);
        }

        // Remove DC spike
        let mid = final_psd.len() / 2;
        final_psd[mid] = (final_psd[mid - 1] + final_psd[mid + 1]) / 2.0;

        let mut psd_recorder = SignalLogger::new(psd_path.to_str().unwrap());
        psd_recorder.record_psd(final_psd);
        println!("Baseline saved.");
        return;
    }
    let mut signal_reader = SignalReader::new(psd_path.to_str().unwrap());
    let mut expected_average = signal_reader.read_psd();
    let mid = expected_average.len() / 2;
    expected_average[mid] = (expected_average[mid - 1] + expected_average[mid + 1]) / 2.0;
    println!("Baseline loaded: {} bins", expected_average.len());

    let mut iq_recorder = SignalLogger::new(signal_config.capture_output.clone().to_str().unwrap());

    let mut matching =
        MatchingEstimator::new(expected_average.clone(), signal_config.search_size.clone());

    loop {
        let (time_stamp, samples_read) = sdr.read_and_timestamp(&mut samples).unwrap();
        let packet = SdrPacketLog::new(time_stamp, samples_read, samples);
        iq_recorder.log_packet(&packet);
        println!(" wrote packet: {} samples", samples_read);

        let power_spectrum = spectrum_analyzer.psd(&mut samples);
        let mut current_average = spectrum_analyzer.spectral_bin_avg(power_spectrum);

        let mid = current_average.len() / 2;
        current_average[mid] = (current_average[mid - 1] + current_average[mid + 1]) / 2.0;

        let estimate = matching.match_estimate_advanced(&mut current_average);

        println!("Estimate {}", estimate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::process::Command;

    use std::f32::consts::PI;

    #[test]
    fn test_downsampler_and_visualize() {
        let input_sample_rate = 3_000_000.0;
        let decimation_factor = 30;
        let target_sample_rate = input_sample_rate / decimation_factor as f32; // 100,000.0
        let chunk_size = 100_000; // 33ms of data
        
        let mut raw_data = vec![Complex::new(0.0, 0.0); chunk_size];

        // Synthetic test
        // We inject a 10kHz signal (should survive) and an 800kHz signal (should be destroyed)
        let f_pass = 10_000.0;
        let f_stop = 800_000.0;

        for i in 0..chunk_size {
            let t = i as f32 / input_sample_rate;
            
            // Complex exponential: e^(j*2*pi*f*t) = cos(2*pi*f*t) + j*sin(2*pi*f*t)
            let pass_val = Complex::new((2.0 * PI * f_pass * t).cos(), (2.0 * PI * f_pass * t).sin());
            let stop_val = Complex::new((2.0 * PI * f_stop * t).cos(), (2.0 * PI * f_stop * t).sin());
            
            // Mix them together
            raw_data[i] = pass_val + stop_val;
        }
        

        let mut downsampler = Downsampler::default();
        let mut decimated_data: Vec<Complex<f32>> = Vec::new();

        for chunk in raw_data.chunks(READ_CHUNK_SIZE) {
            let decimated_chunk = downsampler.downsample(chunk);
            decimated_data.extend(decimated_chunk);
        }
        let expected_min_len = (chunk_size / decimation_factor) - (100_000 / READ_CHUNK_SIZE);
        assert!(
            decimated_data.len() >= expected_min_len, 
            "Downsampler did not produce expected output length"
        );

        let bin_path = "test_output.bin";
        let mut file = File::create(bin_path).expect("Failed to create bin file");
        
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                decimated_data.as_ptr() as *const u8,
                decimated_data.len() * std::mem::size_of::<Complex<f32>>(),
            )
        };
        file.write_all(byte_slice).expect("Failed to write to bin file");

        let venv_python = "/home/supergoodname77/Desktop/Learning/ml/.venv/bin/python";
        let script_path = "/home/supergoodname77/Desktop/Learning/ml/signal/plot_signal.py";

        let img_path = "test_output_plot.png";
        let output = Command::new(venv_python) // Use "python" on Windows
            .args(&[
                script_path, 
                bin_path, 
                &target_sample_rate.to_string(), 
                img_path
            ])
            .output()
            .expect("Failed to execute Python script. Is python3 in PATH and numpy/scipy/matplotlib installed?");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Python visualization script failed:\n{}", stderr);
        }

        println!("Test complete. Review '{}' to visually confirm the 800kHz signal was removed and the 10kHz signal remains cleanly.", img_path);
    }

    #[test]
    // #[ignore = "Requires physical SDR hardware plugged in"]
    fn test_live_fm_capture_and_visualize() {
        // SDR Configuration for local FM
        let frequency = 102.1e6;
        let input_rate = 3_000_000.0;
        let config = RadioConfig {
            frequency, // 100.3 MHz
            sample_rate: input_rate as f64,
            gain: None, 
        };

        let freq = frequency / 1e6;

        let mut sdr = SDR::new(config).expect("Failed to init SDR. Is it plugged in?");
        
        // Configure Downsampler for 200kHz Window (Decimate by 15)
        let decimation_factor = 15;
        let target_sample_rate = input_rate / decimation_factor as f32; // 200,000 Hz
        let mut downsampler = Downsampler::new(input_rate, decimation_factor, 90_000.0);

        // Accumulate exactly 1 second of downsampled data
        let target_samples = target_sample_rate as usize; 
        let mut full_second_buffer: Vec<Complex<f32>> = Vec::with_capacity(target_samples);
        let mut accumulated = 0;

        let title = format!("Capturing 1 second of live RF at {} MHz...", freq);
        println!("{title}");

        while accumulated < target_samples {
            let read_len = match sdr.stream.read(&mut [&mut sdr.read_buffer], 100_000) {
                Ok(len) => len,
                Err(e) => {
                    // Check if the error is specifically an overflow
                    if e.to_string().contains("Overflow") {
                        println!("[WARNING] SDR Overflow! CPU couldn't keep up. Dropping packet...");
                        
                       
                        
                        continue; 
                    } else {
                        panic!("Fatal SDR Read Error: {:?}", e);
                    }
                }
            };

            let decimated_chunk = downsampler.downsample(&sdr.read_buffer[..read_len]);
            
            // Prevent over-filling the exact 1-second requirement
            let space_left = target_samples - accumulated;
            if decimated_chunk.len() > space_left {
                full_second_buffer.extend_from_slice(&decimated_chunk[..space_left]);
                accumulated += space_left;
            } else {
                full_second_buffer.extend_from_slice(&decimated_chunk);
                accumulated += decimated_chunk.len();
            }
        }

        assert_eq!(full_second_buffer.len(), target_samples, "Did not collect exactly 1 second");

        // Save data for python script
        let bin_path = "live_fm_test.bin";
        let mut file = File::create(bin_path).expect("Failed to create bin file");
        
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                full_second_buffer.as_ptr() as *const u8,
                full_second_buffer.len() * std::mem::size_of::<Complex<f32>>(),
            )
        };
        file.write_all(byte_slice).expect("Failed to write binary data");

        let venv_python = "/home/supergoodname77/Desktop/Learning/ml/.venv/bin/python";
        let script_path = "/home/supergoodname77/Desktop/Learning/ml/signal/plot_signal.py";
        let img_path = "live_fm_plot.png";
        
        // True for interactive, false for just png
        let interactive_mode = true; 

        let mut cmd = Command::new(venv_python);
        cmd.args(&[
            script_path, 
            bin_path, 
            "--fs", &target_sample_rate.to_string(),
            "--fc", &config.frequency.to_string(),
            "--cutoff", "90000.0", 
            "--decimation", &decimation_factor.to_string(),
        ]);

        if let Some(g) = config.gain {
            cmd.args(&["--gain", &g.to_string()]);
        } else {
            cmd.arg("--agc");
        }

        if interactive_mode {
            cmd.arg("--interactive");
        } else {
            cmd.args(&["--out", img_path]);
        }

        let output = cmd.output().expect("Failed to launch python script");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Python script failed:\n{}", stderr);
        }

        if interactive_mode {
            println!("Interactive session closed.");
        } else {
            println!("Success! Check '{}' to see the spectrum.", img_path);
        }
    }

    #[test]
    fn test_live_fm_30s_replay() {
        let input_rate = 3_000_000.0;
        let config = RadioConfig {
            frequency: 102.1e6, // 102.1 MHz
            sample_rate: input_rate as f64,
            gain: None, // Keep AGC ON for FM
        };

        let mut sdr = SDR::new(config).expect("Failed to init SDR");
        
        let decimation_factor = 15;
        let target_sample_rate = input_rate / decimation_factor as f32; // 200,000 Hz
        let mut downsampler = Downsampler::new(input_rate, decimation_factor, 90_000.0);

        // Calculate size for exactly 30 seconds of decimated data
        let duration_seconds = 30.0;
        let target_samples = (target_sample_rate * duration_seconds) as usize; 
        
        // At 200kHz, 30s = 6,000,000 Complex<f32> samples. 
        let mut full_buffer: Vec<Complex<f32>> = Vec::with_capacity(target_samples);
        let mut accumulated = 0;

        println!("Capturing 30 seconds of live RF at 102.1 MHz ({} samples)...", target_samples);
        println!("Please wait. This will take exactly 30 seconds of real-world time.");

        while accumulated < target_samples {
            let read_len = match sdr.stream.read(&mut [&mut sdr.read_buffer], 100_000) {
                Ok(len) => len,
                Err(e) => {
                    if e.to_string().contains("Overflow") {
                        continue; 
                    } else {
                        panic!("Fatal SDR Read Error: {:?}", e);
                    }
                }
            };

            let decimated_chunk = downsampler.downsample(&sdr.read_buffer[..read_len]);
            
            let space_left = target_samples - accumulated;
            if decimated_chunk.len() > space_left {
                full_buffer.extend_from_slice(&decimated_chunk[..space_left]);
                accumulated += space_left;
            } else {
                full_buffer.extend_from_slice(&decimated_chunk);
                accumulated += decimated_chunk.len();
            }
        }

        // Write the 48MB buffer to disk
        let bin_path = "live_fm_30s.bin";
        let mut file = File::create(bin_path).expect("Failed to create bin file");
        
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                full_buffer.as_ptr() as *const u8,
                full_buffer.len() * std::mem::size_of::<Complex<f32>>(),
            )
        };
        file.write_all(byte_slice).expect("Failed to write binary data");

        let venv_python = "/home/supergoodname77/Desktop/Learning/ml/.venv/bin/python";
        let script_path = "/home/supergoodname77/Desktop/Learning/ml/signal/replay_signal.py";

        println!("Capture complete. Launching Python live replay...");
        

        let status = Command::new(venv_python)
            .args(&[
                script_path, 
                bin_path, 
                "--fs", &target_sample_rate.to_string(),
                "--fc", &config.frequency.to_string(),
                "--cutoff", "90000.0"
            ])
            .status()
            .expect("Failed to launch python script");

        if !status.success() {
            panic!("Python replay script failed.");
        }

        println!("Replay finished cleanly.");
    }


    use rand::prelude::*;
    use rand_distr::{Normal, Distribution};

    // Helper to generate synthetic time-domain I/Q baseband data with AWGN
    fn generate_iq_baseband(freq_offset_hz: f32, amplitude: f32, noise_level: f32, sample_rate: f32, seed: u64) -> [Complex<f32>; BUFF_SIZE] {
        let mut buf = [Complex::new(0.0, 0.0); BUFF_SIZE];
        
        let mut rng = StdRng::seed_from_u64(seed);
        
        // Create a Gaussian (Normal) distribution with a mean of 0.0 and our noise_level as the standard deviation
        let normal_dist = Normal::new(0.0, noise_level).unwrap();

        for i in 0..TARGET_PACKET_SIZE {
            let t = i as f32 / sample_rate;
            
            let signal = if amplitude > 0.0 {
                Complex::new(
                    (2.0 * PI * freq_offset_hz * t).cos() * amplitude,
                    (2.0 * PI * freq_offset_hz * t).sin() * amplitude,
                )
            } else {
                Complex::new(0.0, 0.0)
            };
            
            // Sample true Gaussian thermal noise for both I and Q channels
            let noise_re = normal_dist.sample(&mut rng);
            let noise_im = normal_dist.sample(&mut rng);
            
            buf[i] = signal + Complex::new(noise_re, noise_im);
        }
        buf
    }

    #[test]
    fn test_end_to_end_pipeline_discrimination() {
        let sample_rate = 100_000.0; 
        let down_size = 64; 
        
        // ---------------------------------------------------------
        // 1. Build the Baseline (Seed 1)
        // ---------------------------------------------------------
        let mut baseline_analyzer = SpectrumAnalyzer::new(down_size, TARGET_PACKET_SIZE);
        let mut baseline_iq = generate_iq_baseband(15_000.0, 1.0, 3.0, sample_rate, 1);
        
        // Run through the exact pipeline used in main.rs
        let baseline_psd = baseline_analyzer.psd(&mut baseline_iq);
        let mut baseline_avg = baseline_analyzer.spectral_bin_avg(baseline_psd);
        
        let mid = baseline_avg.len() / 2;
        baseline_avg[mid] = (baseline_avg[mid - 1] + baseline_avg[mid + 1]) / 2.0;

        let mut estimator = MatchingEstimator::new(baseline_avg.clone(), 50);

        // ---------------------------------------------------------
        // 2. Live Match (Seed 2)
        // ---------------------------------------------------------
        let mut match_analyzer = SpectrumAnalyzer::new(down_size, TARGET_PACKET_SIZE);
        let mut live_match_iq = generate_iq_baseband(15_500.0, 1.0, 3.0, sample_rate, 2);
        
        let live_match_psd = match_analyzer.psd(&mut live_match_iq);
        let mut live_match_avg = match_analyzer.spectral_bin_avg(live_match_psd);
        live_match_avg[mid] = (live_match_avg[mid - 1] + live_match_avg[mid + 1]) / 2.0;

        let score_match = estimator.match_estimate_advanced(&mut live_match_avg);
        println!("End-to-End Shifted Match Score: {}", score_match);

        // ---------------------------------------------------------
        // 3. Live Noise (Seed 3)
        // ---------------------------------------------------------
        let mut noise_analyzer = SpectrumAnalyzer::new(down_size, TARGET_PACKET_SIZE);
        let mut live_noise_iq = generate_iq_baseband(0.0, 0.0, 3.0, sample_rate, 3);
        
        let live_noise_psd = noise_analyzer.psd(&mut live_noise_iq);
        let mut live_noise_avg = noise_analyzer.spectral_bin_avg(live_noise_psd);
        live_noise_avg[mid] = (live_noise_avg[mid - 1] + live_noise_avg[mid + 1]) / 2.0;

        let score_noise = estimator.match_estimate_advanced(&mut live_noise_avg);
        println!("End-to-End Pure Noise Score: {}", score_noise);

        assert!(score_match > 0.6, "Pipeline failed to recognize a shifted signal in noise.");
        assert!(score_noise < 0.3, "Pipeline generated a false positive on flat noise.");
    }
}
