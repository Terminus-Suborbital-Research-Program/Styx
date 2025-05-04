use bincode::{
    config::standard,
    error::{DecodeError, EncodeError},
};

#[cfg(not(feature = "std"))]
use embedded_io::ErrorType;
#[cfg(not(feature = "std"))]
use heapless::Vec;

use crate::packets::ApplicationPacket;

#[cfg(feature = "std")]
use std::fmt::Debug;

#[cfg_attr(not(feature = "std"), derive(defmt::Format))]
pub enum InterfaceError<E> {
    /// Underlying device error
    DeviceError(E),
    /// Decoding error
    DecodeError(#[defmt(Debug2Format)] DecodeError),
    /// Encoding error
    EncodeError(#[defmt(Debug2Format)] EncodeError),
    /// Buffer overflow
    BufferFull,
}

impl<E> From<DecodeError> for InterfaceError<E> {
    fn from(err: DecodeError) -> Self {
        InterfaceError::DecodeError(err)
    }
}

impl<E> From<EncodeError> for InterfaceError<E> {
    fn from(err: EncodeError) -> Self {
        InterfaceError::EncodeError(err)
    }
}

/// Trait for packet I/O operations
pub trait PacketIO {
    /// The associated error type for I/O operations. Must be Debug.
    #[cfg(feature = "std")]
    type Error: Debug;
    /// The associated error type for I/O operations. Must be Debug
    #[cfg(not(feature = "std"))]
    type Error: core::fmt::Debug;

    fn write(&mut self, packet: ApplicationPacket) -> Result<(), InterfaceError<Self::Error>>;
    fn write_into<T: Into<ApplicationPacket>>(
        &mut self,
        packet: T,
    ) -> Result<(), InterfaceError<Self::Error>>;
    fn update(&mut self) -> Result<(), InterfaceError<Self::Error>>;
    fn read_packet(&mut self) -> Result<Option<ApplicationPacket>, InterfaceError<Self::Error>>;
}

#[cfg(feature = "std")]
pub use std_impl::*;
#[cfg(feature = "std")]
mod std_impl {
    use super::*;
    use std::io::{Read, Write};

    pub struct PacketDevice<D> {
        device: D,
        buffer: Vec<u8>,
    }

    impl<D> PacketDevice<D>
    where
        D: Read + Write,
    {
        pub fn new(device: D) -> Self {
            Self {
                device,
                buffer: Vec::new(),
            }
        }
    }

    impl<D> PacketIO for PacketDevice<D>
    where
        D: Read + Write,
    {
        type Error = std::io::Error;

        fn write(&mut self, packet: ApplicationPacket) -> Result<(), InterfaceError<Self::Error>> {
            bincode::encode_into_std_write(packet, &mut self.device, standard())?;
            Ok(())
        }

        fn write_into<T: Into<ApplicationPacket>>(
            &mut self,
            packet: T,
        ) -> Result<(), InterfaceError<Self::Error>> {
            self.write(packet.into())
        }

        fn update(&mut self) -> Result<(), InterfaceError<Self::Error>> {
            let mut buf = [0u8; 1024];
            match self.device.read(&mut buf) {
                Ok(0) => {}
                Ok(bytes) => {
                    self.buffer.extend_from_slice(&buf[..bytes]);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(InterfaceError::DeviceError(e)),
            }
            Ok(())
        }

        fn read_packet(
            &mut self,
        ) -> Result<Option<ApplicationPacket>, InterfaceError<Self::Error>> {
            let (packet, bytes): (ApplicationPacket, usize) =
                match bincode::decode_from_slice(&self.buffer, standard()) {
                    Ok(info) => info,
                    Err(DecodeError::UnexpectedEnd { .. }) => return Ok(None),
                    Err(e) => {
                        if !self.buffer.is_empty() {
                            self.buffer.remove(0);
                        }
                        return Err(InterfaceError::DecodeError(e));
                    }
                };
            self.buffer.drain(0..bytes);
            Ok(Some(packet))
        }
    }
}

#[cfg(not(feature = "std"))]
pub use no_std::*;

#[cfg(not(feature = "std"))]
mod no_std {
    /// A no_std packet device
    pub struct PacketDevice<D, const N: usize> {
        device: D,
        buffer: Vec<u8, N>,
    }

    impl<D, const N: usize> PacketDevice<D, N>
    where
        D: ErrorType,
    {
        pub fn new(device: D) -> Self {
            Self {
                device,
                buffer: Vec::new(),
            }
        }

        pub fn read_packet(
            &mut self,
        ) -> Result<Option<ApplicationPacket>, InterfaceError<D::Error>> {
            let (packet, bytes): (ApplicationPacket, usize) =
                match bincode::decode_from_slice(&self.buffer, standard()) {
                    Ok(info) => info,
                    #[allow(unused_variables)]
                    Err(DecodeError::UnexpectedEnd { additional }) => {
                        return Ok(None);
                    }
                    Err(e) => {
                        if self.buffer.len() > 0 {
                            self.buffer.remove(0);
                        }
                        return Err(InterfaceError::DecodeError(e));
                    }
                };

            for _ in 0..bytes {
                self.buffer.remove(0);
            }

            Ok(Some(packet))
        }
    }

    use super::*;
    use defmt::Format;
    use embedded_io::{Read, ReadReady, Write};

    impl<D, const N: usize> PacketIO for PacketDevice<D, N>
    where
        D: Read + ReadReady + Write,
        D::Error: core::fmt::Debug + Format,
    {
        type Error = D::Error;

        fn write(&mut self, packet: ApplicationPacket) -> Result<(), InterfaceError<Self::Error>> {
            let mut buf = [0u8; N];
            let bytes = bincode::encode_into_slice(packet, &mut buf, standard())?;

            self.device
                .write_all(&buf[..bytes])
                .map_err(InterfaceError::DeviceError)?;
            Ok(())
        }

        fn write_into<T: Into<ApplicationPacket>>(
            &mut self,
            packet: T,
        ) -> Result<(), InterfaceError<Self::Error>> {
            self.write(packet.into())
        }

        fn update(&mut self) -> Result<(), InterfaceError<Self::Error>> {
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

                match self.buffer.extend_from_slice(&buf[..bytes]) {
                    Ok(()) => Ok(()),
                    Err(_) => Err(InterfaceError::BufferFull),
                }
            } else {
                Ok(())
            }
        }

        fn read_packet(
            &mut self,
        ) -> Result<Option<ApplicationPacket>, InterfaceError<Self::Error>> {
            let (packet, bytes): (ApplicationPacket, usize) =
                match bincode::decode_from_slice(&self.buffer, standard()) {
                    Ok(info) => info,
                    #[allow(unused_variables)]
                    Err(DecodeError::UnexpectedEnd { additional }) => {
                        return Ok(None);
                    }
                    Err(e) => {
                        if self.buffer.len() > 0 {
                            self.buffer.remove(0);
                        }
                        return Err(InterfaceError::DecodeError(e));
                    }
                };

            for _ in 0..bytes {
                self.buffer.remove(0);
            }

            Ok(Some(packet))
        }
    }
}
