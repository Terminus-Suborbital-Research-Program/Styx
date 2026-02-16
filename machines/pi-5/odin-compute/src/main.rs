mod networking;
mod tasks;

use std::{
   fs::{File, OpenOptions}, io::{BufReader, BufWriter, Read, Write}, mem, net::{TcpListener, TcpStream, UdpSocket}, thread, time::Duration
};

use bincode::{
    config::{standard, Configuration, LittleEndian, NoLimit, Varint}, error::DecodeError, serde::{decode_from_reader, decode_from_slice, encode_into_slice}
};
use env_logger::{Builder, Target};
use log::{error, info, LevelFilter};
use rtrb::RingBuffer;

use crate::tasks::{
    signal_process::SignalProcessor,
    signal_read::SDRListener, startracker::StartrackerThread,
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

    let sampling_task = SDRListener::begin_sampling(samples_producer).unwrap();

    // Refactor to be one combined call, but not ugly.
    let (startracking_thread, quaternion_reciever) =  StartrackerThread::new();
    let startracking_thread_handle = startracking_thread.begin_startracking();
    

    // start_test_tcp_receiver();
    // verify_recording("sdr_recording.bin");
    start_file_recorder();

    std::thread::sleep(Duration::from_millis(500));

    
    let mut stream = TcpStream::connect("127.0.0.1:7878").expect("Failed to connect to Jupiter");
    stream.set_nonblocking(true).expect("Failed to set non-blocking");
    stream.set_write_timeout(Some(Duration::from_micros(100))).unwrap();
    
    std::thread::sleep(Duration::from_millis(500));

    let (signal_processor, packet_tx, estimate_rx) = SignalProcessor::default();
    
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

fn start_file_recorder() {

    if std::path::Path::new("sdr_recording.bin").exists() {
        let _ = std::fs::remove_file("sdr_recording.bin");
    }

    thread::Builder::new()
        .name("tcp-recorder".into())
        .stack_size(4 * 1024 * 1024) 
        .spawn(move || {
            let listener = TcpListener::bind("127.0.0.1:7878").expect("Failed to bind");
            info!("Recorder listening on 127.0.0.1:7878...");

            let mut buffer = [0u8; BUFF_SIZE * 10];

            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        info!("Connection established: {:?}", stream.peer_addr());

                        let file = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open("sdr_recording.bin")
                            .expect("Failed to open recording file");


                        let mut writer = BufWriter::with_capacity(1 * 1024 * 1024, file);

                        loop {
                            match stream.read(&mut buffer) {
                                Ok(0) => {
                                    info!("Sender disconnected. Closing file.");
                                    break;
                                }
                                Ok(bytes_read) => {
                                    if let Err(e) = writer.write(&buffer[..bytes_read])  {
                                        error!("Error writing encoded data {}", e);
                                    }
                                }

                                Err(e) => {
                                    error!("Error reading from socket{}", e);
                                }
                            }
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
      if !std::path::Path::new(filepath).exists() {
        info!("No recording file found to verify.");
        return;
    }
    
    info!("Verifying recording from: {}", filepath);
    let file = File::open(filepath).expect("File not found");

    let mut reader = BufReader::new(file);

    let packet_size = mem::size_of::<SdrPacketLog>();
    
    let mut buffer = vec![0u8; packet_size];
    let mut count = 0;

    let config = standard();

    loop {
        count += 1;
        match  decode_from_reader::<SdrPacketLog,&mut BufReader<File>,Configuration<LittleEndian, Varint, NoLimit>>(&mut reader, config) {
            Ok(sdr_packet) => {
            info!("Packet #{}: TS={} SampleCount={}", 
                                count, sdr_packet.timestamp, sdr_packet.sample_count);
            }
            Err(e) => {
                error!("File read error: {}", e);
                break;
            }
        } 
    }
}


