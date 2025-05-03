use bincode::{
    config::standard,
    error::{DecodeError, EncodeError},
};
use embedded_io::{Error, ErrorType, Read, Write};
use heapless::Vec;

use crate::ApplicationPacket;

#[derive(Debug)]
pub enum InterfaceError<E: Error> {
    /// Underlying device error
    DeviceError(E),
    /// Decoding error
    DecodeError(DecodeError),
    /// Encoding error
    EncodeError(EncodeError),
    /// Buffer overflow
    BufferFull,
}

impl<E: Error> From<DecodeError> for InterfaceError<E> {
    fn from(err: DecodeError) -> Self {
        InterfaceError::DecodeError(err)
    }
}

impl<E: Error> From<EncodeError> for InterfaceError<E> {
    fn from(err: EncodeError) -> Self {
        InterfaceError::EncodeError(err)
    }
}

/// A device encapsulating a read/write device to send and receive packets
pub struct PacketDevice<D, const N: usize> {
    device: D,
    buffer: Vec<u8, N>,
}

impl<D: ErrorType, const N: usize> PacketDevice<D, N> {
    /// Creates a new interface from a compatible device
    pub fn new(device: D) -> Self {
        Self {
            device,
            buffer: Vec::new(),
        }
    }

    /// Deserialize a packet off the device. Returns Ok(None) if no packet is available,
    /// returns the error otherwise.
    pub fn read_packet(&mut self) -> Result<Option<ApplicationPacket>, InterfaceError<D::Error>> {
        // Attempt to decode a packet
        let (packet, bytes): (ApplicationPacket, usize) =
            match bincode::decode_from_slice(&self.buffer, standard()) {
                Ok(info) => info,
                #[allow(unused_variables)]
                Err(DecodeError::UnexpectedEnd { additional }) => {
                    // Not enough yet, return without removing any bytes
                    return Ok(None);
                }
                Err(e) => {
                    // Decode error, return the error
                    return Err(InterfaceError::DecodeError(e));
                }
            };

        // Remove the bytes from the buffer
        for _ in 0..bytes {
            self.buffer.remove(0);
        }

        Ok(Some(packet))
    }
}

impl<D: Write, const N: usize> PacketDevice<D, N> {
    /// Writes an application packet to the device. Blocks until the packet is read.
    pub fn write(&mut self, packet: ApplicationPacket) -> Result<(), InterfaceError<D::Error>> {
        // Serialize
        let mut buf = [0u8; N];
        let bytes = bincode::encode_into_slice(packet, &mut buf, standard())?;

        // Write to device
        self.device
            .write_all(&buf[..bytes])
            .map_err(InterfaceError::DeviceError)?;
        Ok(())
    }

    /// Write anything that can be turned into an application packet to the device.
    pub fn write_into<T: Into<ApplicationPacket>>(
        &mut self,
        packet: T,
    ) -> Result<(), InterfaceError<D::Error>> {
        self.write(packet.into())
    }
}

impl<D: Read, const N: usize> PacketDevice<D, N> {
    /// Update with the latest data from the device
    pub fn update(&mut self) -> Result<(), InterfaceError<D::Error>> {
        let mut buf = [0u8; N];
        let bytes = self
            .device
            .read(&mut buf)
            .map_err(InterfaceError::DeviceError)?;

        // Append to buffer
        match self.buffer.extend_from_slice(&buf[..bytes]) {
            Ok(()) => Ok(()),
            Err(_) => Err(InterfaceError::BufferFull),
        }
    }
}
