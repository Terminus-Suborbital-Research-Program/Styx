use std::io::{self, Write};
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use bincode::{config::standard, serde::encode_into_slice};
use log::{error, warn};
use rtrb::Consumer;

use signet::record::packet::SdrPacketLog;

pub struct RadioHandler {
    pub consumer: Consumer<SdrPacketLog>,
    pub stream: Option<TcpStream>,
    pub processor_tx: Sender<Box<SdrPacketLog>>,
    pub process_interval: usize,
}

impl RadioHandler {
    pub fn spawn(mut self) -> JoinHandle<()> {
        thread::Builder::new()
            .name("radio-handler".into())
            .spawn(move || {
                let mut packet_buf = vec![0u8; signet::sdr::radio_config::BUFF_SIZE * 10]; 
                let mut frame_count: usize = 0;

                loop {
                    match self.consumer.read_chunk(1) {
                        Ok(mut chunk) => {
                            let packet = &chunk.as_slices().0[0];

                            Self::forward_to_tcp(&mut self.stream, packet, &mut packet_buf);
                            Self::route_to_processor(&self.processor_tx, packet, frame_count, self.process_interval);

                            frame_count = frame_count.wrapping_add(1);
                            chunk.commit(1);
                        }
                        Err(rtrb::chunks::ChunkError::TooFewSlots(_)) => {
                            thread::sleep(Duration::from_micros(200));
                        }
                        Err(e) => error!("RingBuffer Error: {:?}", e),
                    }
                }
            }).unwrap()
    }

    #[inline]
    fn forward_to_tcp(stream_opt: &mut Option<TcpStream>, packet: &SdrPacketLog, buffer: &mut [u8]) {
        // If the network is down, skip encoding and sending entirely
        if let Some(stream) = stream_opt {
            match encode_into_slice(packet, buffer, standard()) {
                Ok(bytes) => {
                    if let Err(e) = stream.write_all(&buffer[..bytes]) {
                        if e.kind() == io::ErrorKind::WouldBlock {
                            warn!("TCP Buffer Full: Dropping packet to maintain real-time flow");
                        } else {
                            error!("TCP Write Error: {}", e);
                        }
                    }
                }
                Err(e) => error!("Encode Error: {}", e),
            }
        }
    }

    #[inline]
    fn route_to_processor(tx: &Sender<Box<SdrPacketLog>>, packet: &SdrPacketLog, frame_count: usize, interval: usize) {
        if frame_count % interval == 0 {
            if let Err(e) = tx.send(Box::new(*packet)) {
                error!("Processor Send Error: Pipeline disconnected - {}", e);
            }
        }   
    }
}