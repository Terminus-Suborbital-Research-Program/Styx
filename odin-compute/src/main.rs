use signet::{
    record::{
        log::{SignalLogger, SignalReader},
        packet::SdrPacket,
    }, sdr::{
        radio_config::{
        BUFF_SIZE, RadioConfig, TARGET_PACKET_SIZE
        
    }, sdr::SDR}, signal::{estimator::MatchingEstimator, signal_config::{self, SignalConfig}, spectrum_analyzer::SpectrumAnalyzer}, tools::cli::{Cli, Commands}
};
use rustfft::num_complex::Complex;

use std::thread;
use rtrb::{RingBuffer, PushError, PopError, PeekError};
mod tasks;
use tasks::signal_read::SDRListener;
fn main() {

    let (mut samples_producer, samples_consumer) = RingBuffer::<Complex<f32>>::new(1_000_000);

    let sampling_task = SDRListener::begin_sampling(samples_producer);
    

    let signal_process_task = thread::spawn(move || {
        samples_consumer.read_chunk(n)
    });

}


