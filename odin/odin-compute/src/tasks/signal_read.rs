
use signet::{
    record::{
        log::{SignalLogger, SignalReader},
        packet::SdrPacketLog,
    }, sdr::{
        radio_config::{
        BUFF_SIZE, RadioConfig, TARGET_PACKET_SIZE
        
    }, sdr::SDR}, signal::{estimator::MatchingEstimator, signal_config::{self, SignalConfig}, spectrum_analyzer::SpectrumAnalyzer}, tools::cli::{Cli, Commands}
};
use rustfft::{num_complex::Complex, num_traits::{Zero, zero}};

use std::thread;
// use rtrb::{RingBuffer, PushError, PopError, PeekError};
use rtrb::{Producer, PushError, PopError, PeekError};




pub struct SDRListener {}

impl SDRListener {

    pub fn begin_sampling(mut samples_producer: Producer<SdrPacketLog>) -> Result<thread::JoinHandle<()>, String> {

        // Initialize hardware and analyzer
        let mini_config = RadioConfig::new(1420.405e6, 3.0e6);
        let signal_config = SignalConfig::default();
        let mut spectrum_analyzer = SpectrumAnalyzer::new(
            signal_config.down_size,
            TARGET_PACKET_SIZE,
        );
        let mut sdr = SDR::new(mini_config).map_err(|s| format!("SDR Not Found {s}"))?;

        // Repeatedly push to spsc with new data
        let signal_read_handle = thread::spawn(move || {
            loop {
                match samples_producer.write_chunk(1) {
                    Ok(mut write_chunk) => {
                        let (slc_1, slc_2) = write_chunk.as_mut_slices();
                        let sdr_packet = &mut slc_1[0];
                        
                        match sdr.read_and_timestamp(&mut sdr_packet.samples) {
                            Ok((timestamp, valid_samples)) => {
                                sdr_packet.sample_count = valid_samples;
                                sdr_packet.timestamp = timestamp;
                                write_chunk.commit(1);
                            }

                            Err(e) => {
                                eprintln!("Error reading signal from SDR: {}",e)
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Consumer too slow, no write chunk available")
                    }
                }
            }
        });
        Ok(signal_read_handle)
    }

}