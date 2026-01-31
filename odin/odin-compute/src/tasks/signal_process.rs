use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use signet::{
    record::{
        packet::SdrPacketLog,
        log::SignalReader,
    },
    signal::{
        estimator::MatchingEstimator,
        spectrum_analyzer::SpectrumAnalyzer,
        signal_config::SignalConfig,

    },
    sdr::radio_config::TARGET_PACKET_SIZE,
};

pub struct SignalProcessor {
    packet_receiver: Receiver<SdrPacketLog>,
    quality_estimate_sender: Sender<f32>,
    spectrum_analyzer: SpectrumAnalyzer,
    matching: MatchingEstimator,
}

impl SignalProcessor{

    pub fn new(
        spectrum_analyzer: SpectrumAnalyzer,
        matching: MatchingEstimator,
    ) -> (Self, Sender<SdrPacketLog>, Receiver<f32>) {
        // Channel for sending raw SDR packets TO the worker
        let (packet_tx, packet_rx) = channel();
        // Channel for receiving quality estimates FROM the worker
        let (estimate_tx, estimate_rx) = channel();

        let processor = Self {
            packet_receiver: packet_rx,
            quality_estimate_sender: estimate_tx,
            spectrum_analyzer,
            matching,
        };

        (processor, packet_tx, estimate_rx)
    }

    // Not true default as it does not implement the trait. This simply hides away the logic of 
    // initiallizing signal processing components like the analyzer and estimator, for convenience
    // of reading in the main loop.
    pub fn default() -> (Self, Sender<SdrPacketLog>, Receiver<f32>) {
        // Initialize resources - where do we get a comparison from, by what factor do 
        // we bin our psd?
        let psd_path = "./comp.psd";
        let signal_config = SignalConfig::default();

        let spectrum_analyzer = SpectrumAnalyzer::new(signal_config.down_size, TARGET_PACKET_SIZE);
        let mut signal_reader = SignalReader::new(psd_path);

        let expected_average = signal_reader.read_psd();
        let matching = MatchingEstimator::new(
            expected_average,
            signal_config.search_size.clone(),
        );

        SignalProcessor::new(spectrum_analyzer, matching)
    }

    pub fn begin_signal_processing(mut self)  -> JoinHandle<()>  {
        let signal_process_task = thread::spawn(move || {
            loop {
                if let Ok(mut sdr_packet) = self.packet_receiver.recv_timeout(Duration::from_micros(100)){
                    // Process
                    let power_spectrum = self.spectrum_analyzer.psd(&mut sdr_packet.samples);
                    let mut current_average = self.spectrum_analyzer.spectral_bin_avg(power_spectrum);

                    // Estimate
                    let estimate = self.matching.match_estimate_advanced(&mut current_average);

                    // Send Results
                    if let Err(e) = self.quality_estimate_sender.send(estimate) {
                        eprintln!("Error sending packet: {}", e);
                    }
                }
            }
        });
        signal_process_task
    }
}


