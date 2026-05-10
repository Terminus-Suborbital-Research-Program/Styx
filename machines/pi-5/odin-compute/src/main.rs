mod networking;
mod tasks;

use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Read, Write},
    mem,
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use bincode::{
    config::{Configuration, LittleEndian, NoLimit, Varint, standard},
    serde::{decode_from_reader, encode_into_slice},
};
use env_logger::Builder;
use log::{LevelFilter, error, info};
use rtrb::RingBuffer;

use crate::tasks::{
    signal_process::SignalProcessor, signal_read::SDRListener, startracker::StartrackerThread,
    radio_handler::RadioHandler, serial_handler::SerialHandler
};

use signet::{
    record::packet::SdrPacketLog,
    sdr::radio_config::BUFF_SIZE,
};

use bin_packets::data::adcs::AttitudeMetrics;
use bin_packets::packets::ApplicationPacket;
use bin_packets::time::Timestamp;


fn main() {
    // env_logger::init();
    Builder::new()
        .filter(None, LevelFilter::max())
        .format(|buf, record| writeln!(buf, "{}: {}", record.level(), record.args()))
        .target(env_logger::Target::Stdout) // Explicitly set the target to stdout
        .init();

    let mut uart_port = serialport::new("/dev/ttyAMA0", 115_200)
        .timeout(Duration::from_millis(10))
        .open()
        .expect("Failed to open UART port");

    let (sdr_samples_producer, mut sdr_samples_consumer) = RingBuffer::<SdrPacketLog>::new(100);

    let _sdr_sampling_task = SDRListener::begin_sampling(sdr_samples_producer).unwrap();

    // PLACEHOLDER
    //let _adcs_sampling_task = SDRListener::begin_sampling(adcs_samples_producer).unwrap();

    // Refactor to be one combined call, but not ugly.
    // Initialize Main Camera Thread
    let (main_tracker, main_quaternion_rx) = StartrackerThread::new("/dev/tevs-main");
    let _main_tracker_handle = main_tracker.begin_startracking();

    // Initialize Aux Camera Thread
    let (aux_tracker, aux_quaternion_rx) = StartrackerThread::new("/dev/tevs-aux");
    let _aux_tracker_handle = aux_tracker.begin_startracking();

    // start_test_tcp_receiver();
    // verify_recording("sdr_recording.bin");
    // start_file_recorder();

    std::thread::sleep(Duration::from_millis(500));

    let sdr_stream = match TcpStream::connect("10.0.0.1:7878") {
        Ok(stream) => {
            stream.set_nonblocking(true).expect("Failed to set non-blocking");
            stream.set_write_timeout(Some(Duration::from_micros(100))).unwrap();
            info!("SDR TCP connected to Jupiter.");
            Some(stream)
        }
        Err(e) => {
            error!("Failed remote connection to Jupiter: {}. Proceeding without SDR telemetry.", e);
            None
        }
    };

    let adcs_stream = match TcpStream::connect("10.0.0.1:7879") {
        Ok(stream) => {
            stream.set_nonblocking(true).expect("Failed to set non-blocking");
            stream.set_write_timeout(Some(Duration::from_micros(100))).unwrap();
            info!("ADCS TCP connected to Jupiter.");
            Some(stream)
        }
        Err(e) => {
            error!("Failed remote connection to Jupiter: {}. Proceeding with local UART only.", e);
            None
        }
    };

    std::thread::sleep(Duration::from_millis(500));

    let (signal_processor, packet_tx, estimate_rx) = SignalProcessor::default();

    signal_processor.begin_signal_processing();
    
    let serial_handler = SerialHandler {
        uart_port,
        adcs_stream, 
        main_q_rx: main_quaternion_rx,
        aux_q_rx: aux_quaternion_rx,
        estimate_rx,
    };
    let serial_thread = serial_handler.spawn();

    let sdr_radio_handler = RadioHandler {
        consumer: sdr_samples_consumer,
        stream: sdr_stream,
        processor_tx: packet_tx, 
        process_interval: 30,
    };
    let sdr_thread = sdr_radio_handler.spawn();


    serial_thread.join().expect("Serial task panicked");
    sdr_thread.join().expect("SDR Radio task panicked");
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

                        let mut writer = BufWriter::with_capacity(1024 * 1024, file);

                        loop {
                            match stream.read(&mut buffer) {
                                Ok(0) => {
                                    info!("Sender disconnected. Closing file.");
                                    break;
                                }
                                Ok(bytes_read) => {
                                    if let Err(e) = writer.write(&buffer[..bytes_read]) {
                                        error!("Error writing encoded data {}", e);
                                    }
                                }

                                Err(e) => {
                                    error!("Error reading from socket{}", e);
                                }
                            }
                        }
                        let _ = writer.flush();
                    }
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

    let _buffer = vec![0u8; packet_size];
    let mut count = 0;

    let config = standard();

    loop {
        count += 1;
        match decode_from_reader::<
            SdrPacketLog,
            &mut BufReader<File>,
            Configuration<LittleEndian, Varint, NoLimit>,
        >(&mut reader, config)
        {
            Ok(sdr_packet) => {
                info!(
                    "Packet #{}: TS={} SampleCount={}",
                    count, sdr_packet.timestamp, sdr_packet.sample_count
                );
            }
            Err(e) => {
                error!("File read error: {}", e);
                break;
            }
        }
    }
}
