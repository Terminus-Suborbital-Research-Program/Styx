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
use bincode::{config::standard, };
use bincode::serde::{encode_into_slice, EncodeError};

//Fake number for now
const JUPITER_ADDRESS: &str = "127.0.0.1:34254";

use std::net::UdpSocket;

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

    // let mut signal_logger = SignalLogger::new(file_path);
    let mut matching = MatchingEstimator::new(
            expected_average,
            signal_config.search_size.clone(),
        );


    let socket = UdpSocket::bind(JUPITER_ADDRESS).unwrap();

    //Note that the operating system may refuse buffers larger than 65507
    // That's slightly smaller than buff size so we may have to consider clipping packets past that
    // limit, or downsampling could fix that problem.
    let mut packet_buf: [u8;BUFF_SIZE] = [0; BUFF_SIZE];
    // let (amt, src) = socket.recv_from(&mut buf)?;
    let signal_process_task = thread::spawn(move || {
        let mut cnt = 0;
        match samples_consumer.read_chunk(1) {
            Ok(mut read_chunk) => {
                let (slc_1, slc_2) = read_chunk.as_mut_slices();
                let sdr_packet = &mut slc_1[0];
                // signal_logger.log_packet(sdr_packet);
                // let  buf: &mut [u8] = ;
                encode_into_slice(&sdr_packet,  packet_buf.as_mut_slice(), standard());
                socket.send(&packet_buf);

                cnt += 1;
                if cnt > 30 {
                    // In theory this should never have samples below target packet size, so this should be valid
                    // but need to recheck later
                    // let mut samples = &mut sdr_packet.samples[..sdr_packet.sample_count];

                    // This is not reresentative of how we should actually do it because we need to downa
                    let power_spectrum = spectrum_analyzer.psd(&mut sdr_packet.samples);
                    let mut current_average = spectrum_analyzer.spectral_bin_avg(power_spectrum);

                    let estimate = matching.match_estimate_advanced(&mut current_average);

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


