mod tasks;
mod networking;

use signet::{
    record::{
        log::{SignalLogger, SignalReader},
        packet::{SdrPacketLog, SdrPacketOwned},
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

use std::{net::UdpSocket, time::Duration};

use std::thread;
use rtrb::{PeekError, PopError, PushError, RingBuffer, chunks::ChunkError};
use tasks::signal_read::SDRListener;

use crate::tasks::signal_process::SignalProcessor;
fn main() {

    let (mut samples_producer, mut samples_consumer) = RingBuffer::<SdrPacketLog>::new(10);

    let sampling_task = SDRListener::begin_sampling(samples_producer);

    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();

    // Note that the operating system may refuse buffers larger than 65507
    // That's slightly smaller than buff size so we may have to consider clipping packets past that
    // limit, or downsampling could fix that problem.
    let mut packet_buf: [u8;BUFF_SIZE] = [0; BUFF_SIZE];

    let (signal_processor, packet_tx, estimate_rx) = SignalProcessor::default();
    // start_test_receiver();
    
    signal_processor.begin_signal_processing();
    let mut cnt = 0;
    loop {
        match samples_consumer.read_chunk(1) {
            Ok(mut read_chunk) => {
                let (slc_1, slc_2) = read_chunk.as_mut_slices();
                let sdr_packet = &mut slc_1[0];
                
                if let Ok(bytes_written) = encode_into_slice(&sdr_packet,  packet_buf.as_mut_slice(), standard()) {
                    if let Err(e) = socket.send(&packet_buf) {
                        eprintln!("Error sending packet: {}", e);
                    }
                } else {
                    eprintln!("Error encoding packet");
                }

                // Unneccessary optimization is the death of reason ; 
                // I'm going to make signal matching an independent thread, and run it by sending over
                // a cloned packet through a channel, so it does not block up the IO task
                // If this causes an untolerable performance increase then I'll add in a ring buffer
                // ARC, or some more complicated setup.
                cnt += 1;
                if cnt > 30 {
                    // In theory this should never have samples below target packet size, so this should be valid
                    // but need to recheck later
                    // let mut samples = &mut sdr_packet.samples[..sdr_packet.sample_count];

                    // Downsampling should be implemented
                    if let Err(e) = packet_tx.send(sdr_packet.clone()) {
                        eprintln!("Error Sending Packet Data {}", e);
                    };


                    cnt = 0;
                }

                read_chunk.commit(1);
            }
            Err(e) => {
                eprintln!("Error getting read chunk {}, likely consuming too fast", e);

                std::thread::sleep(Duration::from_micros(1000));
                // Need to benchmark to see if this case ever comes up, and if so can I introduce minor buffering
                // with a sleep so that read thread can catch up
                // std::thread::sleep(Duration::from_millis(1000));
            }
        }
    }

}



fn start_test_receiver() {
    thread::spawn(move || {
        let receiver_socket = UdpSocket::bind("127.0.0.1:34254").expect("Failed to bind receiver");
        let mut buf = [0u8; BUFF_SIZE];

        println!("Test receiver listening on 127.0.0.1:34254...");

        loop {
            match receiver_socket.recv_from(&mut buf) {
                Ok((amt, _src)) => {
                    // Decode the packet to verify integrity
                    let config = bincode::config::standard();
                    match bincode::serde::decode_from_slice::<SdrPacketOwned, _>(&buf[..amt], config) {
                        Ok((packet, _len)) => {
                            println!("Received packet at time: {}. Samples: {}", 
                                packet.timestamp, 
                                packet.sample_count);
                        }
                        Err(e) => eprintln!("Failed to decode test packet: {}", e),
                    }
                }
                Err(e) => eprintln!("Receiver error: {}", e),
            }
        }
    });
}


