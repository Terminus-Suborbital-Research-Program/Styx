mod networking;
mod tasks;

use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Read, Write},
    mem,
    net::{TcpListener, TcpStream, UdpSocket},
    thread,
    time::Duration,
};

use bincode::{
    config::standard,
    serde::{decode_from_reader, decode_from_slice, encode_into_slice},
};
use env_logger::{Builder, Target};
use log::{error, info, LevelFilter};
use rtrb::RingBuffer;

use crate::tasks::{
    signal_process::SignalProcessor,
    signal_read::SDRListener,
};

use signet::{
    record::packet::{SdrPacketLog, SdrPacketOwned},
    sdr::radio_config::BUFF_SIZE,
};

fn main() {

    // env_logger::init();
    Builder::new()
        .filter(None, LevelFilter::max())
        .format(|buf, record| {
            writeln!(buf, "{}: {}", record.level(), record.args())
        })
        .target(env_logger::Target::Stdout) // Explicitly set the target to stdout
        .init();

    let (mut samples_producer, mut samples_consumer) = RingBuffer::<SdrPacketLog>::new(100);

    let sampling_task = SDRListener::begin_sampling(samples_producer);

    // start_test_tcp_receiver();
    verify_recording("sdr_recording.bin");
    start_file_recorder();

    std::thread::sleep(Duration::from_millis(500));

    
    let mut stream = TcpStream::connect("127.0.0.1:7878").expect("Failed to connect to Jupiter");
    stream.set_nonblocking(true).expect("Failed to set non-blocking");
    stream.set_write_timeout(Some(Duration::from_micros(100))).unwrap();

    // Increase TCP buffer size for throughput
    // let _ = stream.set_write_timeout(Some(Duration::from_micros(100)));

    let (signal_processor, packet_tx, estimate_rx) = SignalProcessor::default();
    // start_test_receiver();
    
    signal_processor.begin_signal_processing();

    let mut packet_buf: [u8;BUFF_SIZE * 10] = [0; BUFF_SIZE * 10];

    
    // Run main IO loop in a thread with larger stack to handle large fixed-size arrays
    let io_handle = thread::Builder::new()
        .name("io-loop".into())
        .stack_size(8 * 1024 * 1024) // 4 MB stack
        .spawn(move || {
            let mut cnt = 0;
            loop {
                match samples_consumer.read_chunk(1) {
                    Ok(mut read_chunk) => {
                        let (slc_1, _slc_2) = read_chunk.as_mut_slices();
                        let sdr_packet = &mut slc_1[0];

                        // let byte_slice = bytes_of(sdr_packet);
                        // let packet: &SdrPacketLog = from_bytes(byte_slice);
                        // assert_eq!(sdr_packet, packet, "Byte casting must be guaranteed");

                        if let Ok(bytes_written) = encode_into_slice(&sdr_packet,  packet_buf.as_mut_slice(), standard()) {
                            // if let Err(e) = socket.send(&packet_buf) {
                            //     error!("Error sending packet: {}", e);
                            // }

                            if let Err(e) = stream.write_all(&packet_buf[..bytes_written]) {
                                error!("Error sending packet: {}", e);
                            }


                        } else {
                            error!("Error encoding packet");
                        }



                        cnt += 1;
                        if cnt % 30 == 0 {
                            if let Err(e) = packet_tx.send(Box::new(sdr_packet.clone())) {
                                error!("Error Sending Packet Data {}", e);
                            };
                            if let Ok(estimate) = estimate_rx.recv_timeout(Duration::from_micros(20)) {
                                info!("Estimate: {}", estimate);
                            };
                        }

                        read_chunk.commit(1);
                    }
                    Err(_e) => {
                        // Producer hasn't produced yet, back off
                        // std::thread::sleep(Duration::from_micros(500));
                    }
                }
            }
        })
        .expect("Failed to spawn IO thread");
    
    // Main thread waits for IO thread (which runs forever)
    io_handle.join().expect("IO thread panicked");


    
}



fn start_test_receiver() {
    thread::spawn(move || {
        let receiver_socket = UdpSocket::bind("127.0.0.1:34254").expect("Failed to bind receiver");
        let mut buf = [0u8; BUFF_SIZE];

        info!("Test receiver listening on 127.0.0.1:34254...");

        loop {
            match receiver_socket.recv_from(&mut buf) {
                Ok((amt, _src)) => {
                    // Decode the packet to verify integrity
                    let config = bincode::config::standard();
                    match bincode::serde::decode_from_slice::<SdrPacketOwned, _>(&buf[..amt], config) {
                        Ok((packet, _len)) => {
                            info!("Received packet at time: {}. Samples: {}", 
                                packet.timestamp, 
                                packet.sample_count);
                        }
                        Err(e) => error!("Failed to decode test packet: {}", e),
                    }
                }
                Err(e) => error!("Receiver error: {}", e),
            }
        }
    });
}


fn start_test_tcp_receiver() {
    thread::Builder::new()
        .name("tcp-receiver".into())
        .stack_size(8 * 1024 * 1024) 
        .spawn(move || {
        // let listener = TcpListener::bind("127.0.0.1:34254").expect("Failed to bind TCP listener");
        let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
        info!("Test TCP receiver listening on 127.0.0.1:34254...");

        // Accept incoming connections
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    info!("New connection established: {:?}", stream.peer_addr());
                    let mut reader = BufReader::new(stream);
                    loop {
                        // std::thread::sleep(Duration::from_micros(500));
                        info!("ALive");
                        if let Ok(sdr_packet_owned)  = decode_from_reader::<SdrPacketLog,_,_>(&mut reader, standard()) {
                            info!("Decoded");

                        } else {
                            // std::thread::sleep(Duration::from_millis(500));

                            error!("Not decoded");
                        }
                    }
                    
                },
                Err(e) => error!("Connection failed: {}", e),
            }
        }
    }).unwrap();
}



fn start_file_recorder() {
    thread::Builder::new()
        .name("tcp-recorder".into())
        .spawn(move || {
            let listener = TcpListener::bind("127.0.0.1:7878").expect("Failed to bind");
            info!("Recorder listening on 127.0.0.1:7878...");

            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        info!("Connection established: {:?}", stream.peer_addr());

                        let file = OpenOptions::new()
                            .create(true)
                            .write(true)
                            .append(true) 
                            .open("sdr_recording.bin")
                            .expect("Failed to open recording file");

                        let mut writer = BufWriter::with_capacity(1 * 1024 * 1024, file);

                        match std::io::copy(&mut stream, &mut writer) {
                            Ok(bytes_count) => {
                                info!("Connection closed. Wrote {} bytes to disk.", bytes_count);
                            },
                            Err(e) => error!("Stream interrupted: {}", e),
                        }
                        
                        let _ = writer.flush();
                    },
                    Err(e) => error!("Connection failed: {}", e),
                }
            }
        })
        .unwrap();
}

fn verify_recording(filepath: &str) {
    info!("Verifying recording from: {}", filepath);
    let file = File::open(filepath).expect("File not found");
    
    thread::Builder::new()
        .name("Verifier".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {

        let mut reader = BufReader::new(file);

        let packet_size = mem::size_of::<SdrPacketLog>();
        
        let mut buffer = vec![0u8; packet_size];
        let mut count = 0;

        loop {
            match reader.read_exact(&mut buffer) {
                Ok(_) => {
                    
                    match decode_from_slice::<SdrPacketLog, _>(&buffer, standard()) {
                        Ok((packet, _len)) => {
                            if count % 10 == 0 {
                                info!("Packet #{}: TS={} SampleCount={}", 
                                    count, packet.timestamp, packet.sample_count);
                            }
                            count += 1;
                        },
                        Err(e) => {
                            error!("Corrupt packet at index {}: {}", count, e);
                            // break;
                        }
                    }
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        info!("End of file reached. Total verified packets: {}", count);
                    } else {
                        error!("File read error: {}", e);
                    }
                    break;
                }
            }
        }

            
    }).unwrap();

    
}


