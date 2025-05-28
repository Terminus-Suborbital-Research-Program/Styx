use bin_packets::packets::ApplicationPacket;
use bincode::{config::standard, encode_into_slice};
use embedded_io::Write;
use heapless::Vec;

use crate::packets::TinyFrame;

/// A buffering writer that is created with a number of consumed items. Each call to `write` will
/// write some of the stream downlinked.
pub struct BufferingWriter<D: Write, const N: usize> {
    device: D,
    write_buffer: Vec<u8, N>,
}

impl<D: Write, const N: usize> BufferingWriter<D, N> {
    pub fn new(device: D) -> Self {
        Self {
            device,
            write_buffer: Vec::new(),
        }
    }

    /// Add a packet onto the writer. This will return an error if there is not enough room to add
    /// the packet on.
    pub fn add(&mut self, packet: &ApplicationPacket) -> Result<(), ()> {
        let mut packet_buffer = [0u8; N];

        let bytes = encode_into_slice(packet, &mut packet_buffer, standard()).unwrap_or(0);
        let mut start = 0;
        while start < bytes {
            let (frame, used_for_frame) =
                TinyFrame::create_from_slice(&packet_buffer[start..bytes]);

            let mut frame_buffer = [0u8; 128 + 5];
            let bytes_written = frame.encode_into_slice(&mut frame_buffer).unwrap();
            start += used_for_frame;

            if self
                .write_buffer
                .extend_from_slice(&frame_buffer[0..bytes_written])
                .is_err()
            {
                return Err(());
            }
        }

        Ok(())
    }

    /// Current number of bytes in buffer waiting to be written
    pub fn waiting(&self) -> usize {
        self.write_buffer.len()
    }

    /// Write up to a certain number of bytes down the write buffer. This will block until all
    /// bytes are written.
    pub fn write(&mut self, max: usize) -> Result<usize, D::Error> {
        let to_write = core::cmp::min(max, self.write_buffer.len());
        self.device.write_all(&self.write_buffer[0..to_write])?;

        self.write_buffer.rotate_left(to_write);
        unsafe {
            self.write_buffer
                .set_len(self.write_buffer.len() - to_write);
        }
        Ok(to_write)
    }
}
