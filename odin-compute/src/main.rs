use signet::{
    record::{
        log::{SignalLogger, SignalReader},
        packet::SdrPacket,
    },
    sdr::{radio_config::RadioConfig, sdr::SDR},
    signal::{estimator::MatchingEstimator, spectrum_analyzer::SpectrumAnalyzer},
    tools::cli::{Cli, Commands},
};

use std::thread;
fn main() {

    let mini_config = RadioConfig::new(101.03e6, 3.0e6);

    let signal_read_handle = thread::spawn(move || {

    });

}


