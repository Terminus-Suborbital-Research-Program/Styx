
use signet::{
    record::{
        log::{SignalLogger, SignalReader},
        packet::SdrPacket,
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

    pub fn begin_sampling(mut samples_producer: Producer<Complex<f32>>) -> thread::JoinHandle<()> {

        let signal_read_handle = thread::spawn(move || {
            let mini_config = RadioConfig::new(1420.405e6, 3.0e6);
            let signal_config = SignalConfig::default();
            let mut sdr = SDR::new(mini_config).unwrap();
            let mut spectrum_analyzer = SpectrumAnalyzer::new(
                signal_config.down_size,
                TARGET_PACKET_SIZE,
            );
            loop {
                match samples_producer.write_chunk(BUFF_SIZE) {
                    Ok(mut write_chunk) => {
                        // First slice is usually the only one populated
                        // if the second one is populated 
                        let (slc_1, slc_2) = write_chunk.as_mut_slices();

                        if slc_2.is_empty() {
                            // let (time_stamp, samples_read) = sdr.read_and_timestamp(slc_1);
                            match sdr.read_and_timestamp(slc_1) {
                                Ok((time_stamp, samples_read)) => {
                                    write_chunk.commit(samples_read);
                                }
                                Err(e) => {
                                    eprintln!("Error reading from sdr: {}", e);
                                }
                            }
                        } else {
                            let mut contiguous_slice: [Complex<f32>;BUFF_SIZE] = [Complex::new(0.0, 0.0);BUFF_SIZE];
                            match sdr.read_and_timestamp(&mut contiguous_slice) {
                                Ok((time_stamp, samples_read)) => {
                                    let middle = slc_1.len();
                                    
                                    if samples_read > middle {
                                        let remainder = samples_read - middle;
                                        slc_1.copy_from_slice(&contiguous_slice[..middle]);
                                        slc_2[..remainder].copy_from_slice(&contiguous_slice[middle..samples_read]);
                                        write_chunk.commit(samples_read);

                                        // Handle case where we only read enough data to fill slice one
                                        // This should never happend but this guard is here just in case
                                    } else {
                                        slc_1[..samples_read].copy_from_slice(&contiguous_slice[..samples_read]);
                                        write_chunk.commit(samples_read);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error reading from sdr: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Consumer too slow, no write chunk available")
                    }
                }
            }
        });
        
        signal_read_handle
    }

}