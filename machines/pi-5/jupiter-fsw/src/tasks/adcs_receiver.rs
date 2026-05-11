use std::io::BufReader;
use std::net::TcpListener;
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};

use log::{error, info};
use bin_packets::packets::ApplicationPacket;

pub struct AdcsReceiverTask {
    pub tx: Sender<ApplicationPacket>,
}

impl AdcsReceiverTask {
    pub fn spawn(self) -> JoinHandle<()> {
        thread::Builder::new()
            .name("jupiter-adcs-rx".into())
            .spawn(move || {
                let listener = TcpListener::bind("0.0.0.0:7879").expect("Failed to bind ADCS port");
                info!("ADCS Receiver listening on 0.0.0.0:7879...");
                let bincode_config = bincode::config::standard();

                for stream in listener.incoming() {
                    if let Ok(stream) = stream {
                        info!("ADCS Connection established from Odin: {:?}", stream.peer_addr());
                        
                        let mut reader = BufReader::new(stream);

                        loop {
                            match bincode::serde::decode_from_reader::<ApplicationPacket, _, _>(&mut reader, bincode_config) {
                                Ok(packet) => {
                                    if let Err(e) = self.tx.send(packet) {
                                        error!("Failed to send ADCS packet to main thread: {}", e);
                                        break; 
                                    }
                                }
                                Err(e) => {
                                    error!("ADCS disconnected or decode error: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                }
            }).unwrap()
    }
}