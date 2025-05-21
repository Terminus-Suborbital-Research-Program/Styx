#[cfg(feature = "std")]
pub mod std;

use bincode::{
    config::standard,
    decode_from_slice, encode_into_slice,
    error::{DecodeError, EncodeError},
};
use embedded_io::{Read, ReadReady, Write, WriteReady};

use crate::packets::ApplicationPacket;

/// A device that can read packets
pub trait PacketReader {
    /// Read a packet off the buffer. This may block, depending on the underlying implimentation.
    /// If a packet is returned, this returns Ok(Some)
    fn read(&mut self) -> Option<ApplicationPacket>;
}

/// A device that may block if it tried to read a packet
pub trait BlockingReader {
    fn would_block(&mut self) -> bool;
}

impl<D: ReadReady> BlockingReader for D {
    fn would_block(&mut self) -> bool {
        self.read_ready().unwrap_or(true)
    }
}

/// A wrapped device that will not block
pub trait NonBlockingReader: PacketReader + BlockingReader {
    /// Read a packet, non-blocking
    fn read_non_blocking(&mut self) -> Option<ApplicationPacket> {
        match self.would_block() {
            true => None,
            false => self.read(),
        }
    }
}

impl<D> NonBlockingReader for D where D: PacketReader + BlockingReader {}

/// A packet device running on embedded_io::Read device
pub struct Device<D, const N: usize> {
    device: D,
    buffer: heapless::Vec<u8, N>,
}

impl<D, const N: usize> Device<D, N> {
    /// Create a new reader from a device
    pub fn new(device: D) -> Self {
        Self {
            device,
            buffer: heapless::Vec::new(),
        }
    }

    /// Remove bytes from the front of the buffer
    pub fn drain_some(&mut self, bytes: usize) {
        let bytes = core::cmp::min(bytes, self.buffer.len());

        if bytes > 0 {
            self.buffer.rotate_left(bytes)
        };
    }
}

impl<D, const N: usize> Device<D, N>
where
    D: Read,
{
    /// Read data into the buffer, may block.
    pub fn update(&mut self) {
        let len = self.buffer.len();
        let capacity = self.buffer.capacity();
        let buffer_section = &mut self.buffer[len..capacity];

        // Read into
        if let Ok(bytes_read) = self.device.read(buffer_section) {
            // Exend the buffer by that amount we can do so without checking, because
            // we can never extend by more byes than we have free.
            unsafe {
                self.buffer.set_len(capacity + bytes_read);
            }
        }
    }
}

impl<D: Read, const N: usize> From<D> for Device<D, N> {
    fn from(value: D) -> Self {
        Self::new(value)
    }
}

impl<D: Read, const N: usize> PacketReader for Device<D, N> {
    fn read(&mut self) -> Option<ApplicationPacket> {
        // Buffer in data to try and decode.
        self.update();

        // Potential packet
        let mut pot: Option<ApplicationPacket> = None;
        let mut used_bytes = 0;

        for start in 0..self.buffer.len() {
            let section = &self.buffer[start..self.buffer.len()];
            let res: Result<(ApplicationPacket, usize), DecodeError> =
                decode_from_slice(section, standard());

            match res {
                Ok(packet_size) => {
                    let (packet, bytes_taken) = packet_size;
                    pot = Some(packet);
                    // Remove the bytes that were read off, too
                    used_bytes = start + bytes_taken;
                    break;
                }

                #[allow(unused_variables)] // Wish there were a less awkward way of doing this
                Err(DecodeError::UnexpectedEnd { additional }) => {
                    // Take off however many junk bytes we saw
                    used_bytes = start;
                    break; // No more reading would do anything for us
                }

                _ => {
                    // Do nothing, just drop those bytes from the front and try to continue
                }
            }
        }

        // Clear off the number of bytes we used
        self.drain_some(used_bytes);

        pot
    }
}

impl<D: Read, const N: usize> BlockingReader for Device<D, N>
where
    D: BlockingReader,
{
    fn would_block(&mut self) -> bool {
        self.device.would_block()
    }
}

/// A device that can write packets
pub trait PacketWriter {
    /// Write a packet to the buffer. This may block, depending on the underlying implimentation.
    /// If the packet fails to encode, that is returned
    fn write<T: Into<ApplicationPacket>>(&mut self, packet: T) -> Result<(), EncodeError>;
}

/// A device that may block if it were to try and read
pub trait BlockingWriter {
    /// Returns true if the device would block
    fn would_block(&mut self) -> bool;
}

impl<D> BlockingWriter for D
where
    D: WriteReady,
{
    fn would_block(&mut self) -> bool {
        self.write_ready().unwrap_or(true)
    }
}

impl<D, const N: usize> BlockingWriter for Device<D, N>
where
    D: BlockingWriter,
{
    fn would_block(&mut self) -> bool {
        self.device.would_block()
    }
}

/// A blocking type error, to avoid losing packets if possible
#[derive(Debug, defmt::Format)]
pub enum BlockingWriteError {
    /// Sending would block
    WouldBlock(ApplicationPacket),
    /// An encoding error occured
    EncodeError(#[defmt(Debug2Format)] EncodeError),
}

impl From<EncodeError> for BlockingWriteError {
    fn from(value: EncodeError) -> Self {
        Self::EncodeError(value)
    }
}

impl From<ApplicationPacket> for BlockingWriteError {
    fn from(value: ApplicationPacket) -> Self {
        Self::WouldBlock(value)
    }
}

/// A non-blocking writer
pub trait NonBlockIngWriter: PacketWriter + BlockingWriter {
    /// Write non-blocking
    fn write_non_blocking(&mut self, packet: ApplicationPacket) -> Result<(), BlockingWriteError> {
        match self.would_block() {
            false => Ok(self.write(packet)?),

            true => Err(packet.into()),
        }
    }
}

impl<D, const N: usize> PacketWriter for Device<D, N>
where
    D: Write,
{
    fn write<T: Into<ApplicationPacket>>(&mut self, packet: T) -> Result<(), EncodeError> {
        let mut buf = [0u8; N];
        match encode_into_slice(packet.into(), &mut buf, standard()) {
            Ok(written) => {
                self.device.write(&buf[0..written]).ok();
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

impl<D> NonBlockIngWriter for D where D: PacketWriter + BlockingWriter {}
