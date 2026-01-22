use signet::{
    record::{
        log::{SignalLogger, SignalReader},
        packet::SdrPacketLog,
    }, sdr::{
        radio_config::{
        BUFF_SIZE, RadioConfig, TARGET_PACKET_SIZE
        
    }, sdr::SDR}, signal::{estimator::MatchingEstimator, signal_config::{self, SignalConfig}, spectrum_analyzer::{self, SpectrumAnalyzer}}, tools::cli::{Cli, Commands}
};
use rustfft::num_complex::Complex;

use std::thread;
use rtrb::{RingBuffer, PushError, PopError, PeekError};
mod tasks;
use tasks::signal_read::SDRListener;
fn main() {

    // Ring buffer size at 10 packets, but we'll see if that
    // Add in flag for _logged if there's a significant mismatch between signal
    // process and packets log?
    // Put in packet number to guarantee order? (Maybe unneccessary because that
    // could populate really fast and we also have order from time)
    // Though that would mean in post process we might have to sort by time
    // unless we can guarantee that packets are processed in order despite ringbuffer 
    // looping around?
    let (mut samples_producer, mut samples_consumer) = RingBuffer::<SdrPacketLog>::new(10);

    let sampling_task = SDRListener::begin_sampling(samples_producer);

    let signal_config = SignalConfig::default();
    let mut spectrum_analyzer = SpectrumAnalyzer::new(signal_config.down_size, TARGET_PACKET_SIZE);
    // let estimator = MatchingEstimator::new(current_power_spectrum, expected_power_spectrum, max_shift);
    let file_path = "sdr_packets.dat";
    let psd_path = "cass_a.psd";
    let mut signal_reader = SignalReader::new(psd_path);
    let expected_average = signal_reader.read_psd();

    let mut signal_logger = SignalLogger::new(file_path);
    let mut cnt = 0;
    let signal_process_task = thread::spawn(move || {

        match samples_consumer.read_chunk(1) {
            Ok(mut read_chunk) => {
                let (slc_1, slc_2) = read_chunk.as_mut_slices();
                let sdr_packet = &mut slc_1[0];
                signal_logger.log_packet(sdr_packet);

                let mut samples = &mut sdr_packet.samples[..sdr_packet.sample_count];
                cnt += 1;
                if cnt > 30 {
                    let power_spectrum = spectrum_analyzer.psd(&mut samples);
                    let current_average = spectrum_analyzer.spectral_bin_avg(power_spectrum);
                     let mut matching = MatchingEstimator::new(
                        current_average,
                        expected_average.clone(),
                        signal_config.search_size.clone(),
                    );
                    let estimate = matching.match_estimate_advanced();

                    println!("Estimate {}", estimate);
                    cnt = 0;
                }


            }

            Err(e) => {
                eprintln!("Error getting read chunk {}", e)
            }
        }
    });

}


