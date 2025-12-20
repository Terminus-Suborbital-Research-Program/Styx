use bincode::{Decode, Encode};
use soapysdr::{Device, Direction, RxStream};
use core::{f32, num};
use std::{fs, io::{BufWriter, Write, BufReader}};
use bincode::{config::standard, encode_into_slice, decode_from_slice, serde::{encode_into_std_write, decode_from_std_read} };
use rustfft::num_complex::Complex;



use sig::{
    signal::{estimator::MatchingEstimator, spectrum_analyzer::SpectrumAnalyzer},
    tools::cli::{Cli, Commands},
    sdr::{
        radio_config::RadioConfig,
        sdr::SDR
    },
    record::{
        log::{
            SignalLogger,
            SignalReader,
        },
        packet::SdrPacket,
    }
};

fn main() {
    let mut expected_average: Vec<f32> = Vec::new();

    let (radio_config, signal_config) = Cli::get_configs();
    let (record_baseline, psd_path) = Cli::run_commands();

    let mut sdr = SDR::new(radio_config).unwrap();
    let mut spectrum_analyzer = SpectrumAnalyzer::new(signal_config.down_size, radio_config.target_packet_size.clone());

    let mut accumulator: Vec<Complex<f32>> = Vec::with_capacity(radio_config.target_packet_size);

    if record_baseline {
        let mut psd_recorder = SignalLogger::new(psd_path);
        let _ = sdr.fill_buffer(&mut accumulator);
        let power_spectrum = spectrum_analyzer.psd(&mut accumulator);
        let power_spectrum_bin_averaged = spectrum_analyzer.spectral_bin_avg(power_spectrum);

        psd_recorder.record_psd(power_spectrum_bin_averaged);
        return;
    }
    let mut signal_reader = SignalReader::new(psd_path);
    let expected_average = signal_reader.read_psd();
    println!("Baseline loaded: {} bins", expected_average.len());

    let mut iq_recorder = SignalLogger::new(signal_config.capture_output.clone());

    loop {
        let timestamp = sdr.fill_buffer(&mut accumulator).unwrap();
        let packet = SdrPacket::new(timestamp, &accumulator);
        iq_recorder.log_packet(packet);
        println!(" wrote packet: {} samples", accumulator.len());

        let power_spectrum = spectrum_analyzer.psd(&mut accumulator);
        let current_average = spectrum_analyzer.spectral_bin_avg(power_spectrum);
       
        let mut matching = MatchingEstimator::new(current_average, expected_average.clone(), signal_config.search_size.clone());
        let estimate = matching.match_estimate_advanced();
        
        println!("Estimate {}", estimate);
        accumulator.clear();
    }
}
        
