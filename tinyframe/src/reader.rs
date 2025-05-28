use bin_packets::packets::ApplicationPacket;
use bincode::{config::standard, decode_from_slice};
use defmt::{error, info};
use embedded_io::{Read, ReadReady};
use heapless::Vec;

use crate::packets::{FrameError, TinyFrame};

/// A buffered reader allowing for asynchronus reaading of packets
pub struct BufferedReader<D: Read + ReadReady, const N: usize> {
    device: D,
    incoming_buffer: Vec<u8, N>,
    incoming_frame_buffer: Vec<u8, N>,
}

impl<D: Read + ReadReady, const N: usize> BufferedReader<D, N> {
    pub fn new(device: D) -> Self {
        Self {
            device,
            incoming_buffer: Vec::new(),
            incoming_frame_buffer: Vec::new(),
        }
    }

    pub fn frame_buffer(&self) -> &[u8] {
        &self.incoming_frame_buffer
    }

    pub fn packet_buffer(&self) -> &[u8] {
        &self.incoming_buffer
    }

    pub fn update(&mut self) -> Result<(), D::Error> {
        if self.device.read_ready()? {
            let mut buffer = [0u8; N];
            let bytes = self.device.read(&mut buffer)?;
            self.incoming_frame_buffer
                .extend_from_slice(&buffer[0..bytes])
                .ok();
        }

        // Update the deframed data if possible
        while !self.incoming_frame_buffer.is_empty() {
            match TinyFrame::decode_from_slice(&self.incoming_frame_buffer) {
                Ok((frame, bytes_used)) => {
                    self.incoming_buffer.extend_from_slice(frame.data()).ok();
                    self.incoming_frame_buffer.rotate_left(bytes_used);
                    self.incoming_frame_buffer
                        .truncate(self.incoming_frame_buffer.len() - bytes_used);
                }

                Err(e) => {
                    match e {
                        FrameError::UnexpectedEOF => return Ok(()),

                        _ => {
                            // Any other is  a failure
                            self.incoming_frame_buffer.remove(0);
                            error!("Decoding error on frame! {}", e);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn next_packet(&mut self) -> Option<ApplicationPacket> {
        while !self.incoming_buffer.is_empty() {
            match decode_from_slice::<ApplicationPacket, _>(&self.incoming_buffer, standard()) {
                Ok(packet) => {
                    let new_len = self.incoming_buffer.len() - packet.1;
                    self.incoming_buffer.rotate_left(packet.1);
                    self.incoming_buffer.truncate(new_len);
                    return Some(packet.0);
                }

                #[allow(unused_variables)] // No way to get around this rip
                Err(bincode::error::DecodeError::UnexpectedEnd { additional }) => {
                    return None;
                }

                _ => {
                    // Remove first byte
                    self.incoming_buffer.remove(0);
                }
            }
        }
        None
    }
}

/// Iterate over incoming packets
impl<D: Read + ReadReady, const N: usize> core::iter::Iterator for BufferedReader<D, N> {
    type Item = ApplicationPacket;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_packet()
    }
}
