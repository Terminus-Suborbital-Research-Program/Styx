use rustfft::num_complex::Complex;
use signet::{
    record::{
        log::{SignalLogger, SignalReader},
        packet::SdrPacketLog,
    },
    sdr::{radio_config::{
        RadioConfig,
        TARGET_PACKET_SIZE,
        BUFF_SIZE
        
    }, sdr::SDR},
    signal::{estimator::MatchingEstimator, spectrum_analyzer::SpectrumAnalyzer},
    tools::cli::{Cli, Commands},
};

fn main() {
    let mut expected_average: Vec<f32> = Vec::new();

    let (radio_config, signal_config) = Cli::get_configs();
    let (record_baseline, psd_path) = Cli::run_commands();

    let mut sdr = SDR::new(radio_config).unwrap();
    let mut spectrum_analyzer = SpectrumAnalyzer::new(
        signal_config.down_size,
        TARGET_PACKET_SIZE,
    );

    // let mut accumulator: Vec<Complex<f32>> = Vec::with_capacity(radio_config.target_packet_size + radio_config.read_chunk_size);

    let mut samples: [Complex<f32>;BUFF_SIZE] = [Complex::new(0.0, 0.0); BUFF_SIZE];
    if record_baseline {
        let mut psd_recorder = SignalLogger::new(psd_path.to_str().unwrap());
        let (time_stamp, samples_read) = sdr.read_and_timestamp(&mut samples).unwrap();
        let power_spectrum = spectrum_analyzer.psd(&mut samples);
        let power_spectrum_bin_averaged = spectrum_analyzer.spectral_bin_avg(power_spectrum);

        psd_recorder.record_psd(power_spectrum_bin_averaged);
        return;
    }
    let mut signal_reader = SignalReader::new(psd_path.to_str().unwrap());
    let expected_average = signal_reader.read_psd();
    println!("Baseline loaded: {} bins", expected_average.len());

    let mut iq_recorder = SignalLogger::new(signal_config.capture_output.clone().to_str().unwrap());

    loop {
        let (time_stamp, samples_read) = sdr.read_and_timestamp(&mut samples).unwrap();
        let packet = SdrPacketLog::new(time_stamp, samples_read,samples);
        iq_recorder.log_packet(&packet);
        println!(" wrote packet: {} samples", samples_read);

        let power_spectrum = spectrum_analyzer.psd(&mut samples);
        let current_average = spectrum_analyzer.spectral_bin_avg(power_spectrum);

        let mut matching = MatchingEstimator::new(
            current_average,
            expected_average.clone(),
            signal_config.search_size.clone(),
        );
        let estimate = matching.match_estimate_advanced();

        println!("Estimate {}", estimate);
    }
}
