use std::fs::OpenOptions;
use std::io::{BufWriter, Read, Write};
use std::net::TcpListener;
use std::thread::{self, JoinHandle};

use log::{error, info};
use signet::sdr::radio_config::BUFF_SIZE;

pub struct SdrReceiverTask;

impl SdrReceiverTask {
    pub fn spawn() -> JoinHandle<()> {
        thread::Builder::new()
            .name("jupiter-sdr-rx".into())
            .spawn(|| {
                let listener = TcpListener::bind("0.0.0.0:7878").expect("Failed to bind SDR port");
                info!("SDR Receiver listening on 0.0.0.0:7878...");

                let home = String::from("/home/terminus");
                // let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                let sdr_dir = format!("{}/data/sdr_packets", home);
                std::process::Command::new("mkdir").arg(&sdr_dir).arg("-p").status().ok();
                
                let mut max = 0;
                if let Ok(entries) = std::fs::read_dir(&sdr_dir) {
                    for entry in entries.flatten() {
                        if let Ok(num) = entry.file_name().to_string_lossy().parse::<u32>() {
                            if num > max { max = num; }
                        }
                    }
                }
                let sdr_filepath = format!("{}/{}", sdr_dir, max + 1);

                let mut buffer = [0u8; BUFF_SIZE * 10];

                for stream in listener.incoming() {
                    if let Ok(mut stream) = stream {
                        info!("SDR Connection established from Odin: {:?}", stream.peer_addr());
                        
                        let file = OpenOptions::new().write(true).create(true).truncate(true).open(&sdr_filepath).unwrap();
                        let mut writer = BufWriter::with_capacity(1024 * 1024, file);

                        loop {
                            match stream.read(&mut buffer) {
                                Ok(0) => {
                                    info!("Odin SDR disconnected.");
                                    break;
                                }
                                Ok(bytes_read) => {
                                    if let Err(e) = writer.write_all(&buffer[..bytes_read]) {
                                        error!("SDR Disk Write Error: {}", e);
                                    }
                                }
                                Err(e) => {
                                    error!("SDR Socket Read Error: {}", e);
                                    break;
                                }
                            }
                        }
                        let _ = writer.flush();
                    }
                }
            }).unwrap()
    }
}