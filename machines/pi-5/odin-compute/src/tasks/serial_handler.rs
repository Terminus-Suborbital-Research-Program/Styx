use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::Receiver;
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

use bincode::{config::standard, encode_into_slice};
use log::{error, warn};

use bin_packets::data::adcs::AttitudeMetrics;
use bin_packets::time::Timestamp;
use aether::attitude::Quaternion;
use aether::reference_frame::{Body, ICRF};

pub struct SerialHandler {
    pub uart_port: Box<dyn serialport::SerialPort>,
    pub adcs_stream: TcpStream, 
    pub main_q_rx: Receiver<Quaternion<f32, ICRF<f32>, Body<f32>>>,
    pub aux_q_rx: Receiver<Quaternion<f32, ICRF<f32>, Body<f32>>>,
    pub estimate_rx: Receiver<f32>,
}

impl SerialHandler {
    pub fn spawn(mut self) -> JoinHandle<()> {
        thread::Builder::new()
            .name("serial-handler".into())
            .spawn(move || {
                let mut tx_buffer = [0u8; 1000]; // For encoding AttitudeMetrics down to odin pico
                let mut rx_buffer = [0u8; 1024]; // For incoming ApplicationPackets from odin pico

                loop {
                    // Read from UART and push to Jupiter via TCP
                    self.forward_uart_to_tcp(&mut rx_buffer);

                    // Drain local channels, encode, and send to odin pico via UART
                    self.route_local_to_uart(&mut tx_buffer);
                }
            }).unwrap()
    }

    /// Forward application packet bytes from the odinpico to the ADCS TCP stream.
    #[inline]
    fn forward_uart_to_tcp(&mut self, rx_buffer: &mut [u8]) {
        // Check fi timeout throttles loop or not
        match self.uart_port.read(rx_buffer) {
            Ok(bytes_read) if bytes_read > 0 => {
                if let Err(e) = self.adcs_stream.write_all(&rx_buffer[..bytes_read]) {
                    if e.kind() == io::ErrorKind::WouldBlock {
                        warn!("ADCS TCP Blocked: Dropping telemetry frame");
                    } else {
                        error!("ADCS TCP Write Error: {}", e);
                    }
                }
            }
            Ok(_) => {} // 0 bytes read
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {} // Expected idle timeout
            Err(e) => error!("UART Read Error: {}", e),
        }
    }

    /// Get latest metrics and send to odin pico
    #[inline]
    fn route_local_to_uart(&mut self, tx_buffer: &mut [u8]) {
        let mut main_q = None;
        while let Ok(q) = self.main_q_rx.try_recv() { main_q = Some([q.w(), q.i(), q.j(), q.k()]); }

        let mut aux_q = None;
        while let Ok(q) = self.aux_q_rx.try_recv() { aux_q = Some([q.w(), q.i(), q.j(), q.k()]); }

        let mut estimate = None;
        while let Ok(est) = self.estimate_rx.try_recv() { estimate = Some(est); }

        // If no new data was generated locally, skip encoding
        if main_q.is_none() && aux_q.is_none() && estimate.is_none() { return; }

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        let metrics = AttitudeMetrics {
            timestamp: Timestamp::new(timestamp),
            main_quaternion: main_q,
            aux_quaternion: aux_q,
            signal_match: estimate,
        };

        match encode_into_slice(metrics, tx_buffer, standard()) {
            Ok(bytes) => {
                if let Err(e) = self.uart_port.write_all(&tx_buffer[..bytes]) {
                    error!("UART Write Error: {}", e);
                }
            }
            Err(e) => error!("Encode Error: AttitudeMetrics - {}", e),
        }
    }
}