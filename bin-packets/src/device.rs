use bincode::{
    config::standard,
    error::{DecodeError, EncodeError},
};
use embedded_io::{Error, ErrorType};
use heapless::Vec;

use crate::packets::ApplicationPacket;

#[derive(Debug, defmt::Format)]
pub enum InterfaceError<E: Error> {
    /// Underlying device error
    DeviceError(E),
    /// Decoding error
    DecodeError(#[defmt(Debug2Format)] DecodeError),
    /// Encoding error
    #[defmt(Display2Format)]
    EncodeError(#[defmt(Debug2Format)] EncodeError),
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

/// Trait for packet I/O operations
pub trait PacketIO<E: Error> {
    /// Writes an application packet to the device. Blocks until the packet is read.
    fn write(&mut self, packet: ApplicationPacket) -> Result<(), InterfaceError<E>>;
    /// Write anything that can be turned into an application packet to the device.
    fn write_into<T: Into<ApplicationPacket>>(
        &mut self,
        packet: T,
    ) -> Result<(), InterfaceError<E>>;
    /// Update with the latest data from the device
    fn update(&mut self) -> Result<(), InterfaceError<E>>;
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
                    // Decode error, return the error, after popping off the first byte
                    if self.buffer.len() > 0 {
                        self.buffer.remove(0);
                    }
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

#[cfg(not(feature = "std"))]
mod no_std_impl {
    // Move imports here
    use super::*;
    use embedded_io::{Read, ReadReady, Write};

    impl<D: Read + ReadReady + Write, const N: usize> PacketIO<D::Error> for PacketDevice<D, N> {
        /// Writes an application packet to the device. Blocks until the packet is read.
        fn write(&mut self, packet: ApplicationPacket) -> Result<(), InterfaceError<D::Error>> {
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
        fn write_into<T: Into<ApplicationPacket>>(
            &mut self,
            packet: T,
        ) -> Result<(), InterfaceError<D::Error>> {
            self.write(packet.into())
        }

        /// Update with the latest data from the device
        fn update(&mut self) -> Result<(), InterfaceError<D::Error>> {
            if self
                .device
                .read_ready()
                .map_err(InterfaceError::DeviceError)?
            {
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
            } else {
                Ok(())
            }
        }
    }
}

#[cfg(feature = "std")]
mod std_impl {
    use super::*;
    // Import Error and ErrorKind from embedded_io
    use embedded_io::{Error, ErrorKind};
    use std::io;

    // Define a newtype wrapper around std::io::Error
    #[derive(Debug)]
    pub struct StdIoError(io::Error);

    // Implement From<std::io::Error> for the wrapper
    impl From<io::Error> for StdIoError {
        fn from(err: io::Error) -> Self {
            StdIoError(err)
        }
    }

    // Implement embedded_io::Error for the wrapper type
    impl Error for StdIoError {
        fn kind(&self) -> ErrorKind {
            match self.0.kind() {
                // Access the inner io::Error
                io::ErrorKind::NotFound => ErrorKind::NotFound,
                io::ErrorKind::PermissionDenied => ErrorKind::PermissionDenied,
                io::ErrorKind::ConnectionRefused => ErrorKind::ConnectionRefused,
                io::ErrorKind::ConnectionReset => ErrorKind::ConnectionReset,
                io::ErrorKind::ConnectionAborted => ErrorKind::ConnectionAborted,
                io::ErrorKind::NotConnected => ErrorKind::NotConnected,
                io::ErrorKind::AddrInUse => ErrorKind::AddrInUse,
                io::ErrorKind::AddrNotAvailable => ErrorKind::AddrNotAvailable,
                io::ErrorKind::BrokenPipe => ErrorKind::BrokenPipe,
                io::ErrorKind::AlreadyExists => ErrorKind::AlreadyExists,
                // Map WouldBlock to Other as it doesn't exist in embedded_io::ErrorKind
                io::ErrorKind::WouldBlock => ErrorKind::Other,
                io::ErrorKind::InvalidInput => ErrorKind::InvalidInput,
                io::ErrorKind::InvalidData => ErrorKind::InvalidData,
                io::ErrorKind::TimedOut => ErrorKind::TimedOut,
                io::ErrorKind::WriteZero => ErrorKind::WriteZero,
                io::ErrorKind::Interrupted => ErrorKind::Interrupted,
                io::ErrorKind::Unsupported => ErrorKind::Unsupported,
                // Map UnexpectedEof to Other as it doesn't exist in embedded_io::ErrorKind
                io::ErrorKind::UnexpectedEof => ErrorKind::Other,
                io::ErrorKind::OutOfMemory => ErrorKind::OutOfMemory,
                // Map other std::io::ErrorKind variants to ErrorKind::Other
                _ => ErrorKind::Other,
            }
        }
    }

    // Implement PacketIO for PacketDevice where D implements std::io::Read and std::io::Write
    // Use the StdIoError wrapper type
    impl<D: io::Read + io::Write, const N: usize> PacketIO<StdIoError> for PacketDevice<D, N> {
        /// Writes an application packet to the device using std::io::Write. Blocks until the packet is written.
        fn write(&mut self, packet: ApplicationPacket) -> Result<(), InterfaceError<StdIoError>> {
            // Serialize
            let mut buf = [0u8; N];
            let bytes = bincode::encode_into_slice(packet, &mut buf, standard())?;

            // Write to device using std::io::Write
            self.device
                .write_all(&buf[..bytes])
                .map_err(|e| InterfaceError::DeviceError(StdIoError::from(e)))?; // Map io::Error to StdIoError
            Ok(())
        }

        /// Write anything that can be turned into an application packet to the device using std::io::Write.
        fn write_into<T: Into<ApplicationPacket>>(
            &mut self,
            packet: T,
        ) -> Result<(), InterfaceError<StdIoError>> {
            self.write(packet.into())
        }

        /// Update with the latest data from the device using std::io::Read. Blocks until data is read.
        fn update(&mut self) -> Result<(), InterfaceError<StdIoError>> {
            // std::io::Read is blocking, so we don't need read_ready
            let mut buf = [0u8; N]; // Use a temporary buffer for reading
            let bytes = self
                .device
                .read(&mut buf) // This will block until data is available or an error occurs
                .map_err(|e| InterfaceError::DeviceError(StdIoError::from(e)))?; // Map io::Error to StdIoError

            // Append to internal buffer
            match self.buffer.extend_from_slice(&buf[..bytes]) {
                Ok(()) => Ok(()),
                Err(_) => Err(InterfaceError::BufferFull),
            }
        }
    }
}
