use std::io::{Read, Write};

use bincode::{config::standard, decode_from_slice, encode_into_std_write, error::DecodeError};

use crate::packets::ApplicationPacket;

use super::{PacketReader, PacketWriter};

/// A vector-buffered Read/Write device
pub struct Device<D> {
    device: D,
    buffer: Vec<u8>,
}

impl<D> Device<D> {
    pub fn new(device: D) -> Self {
        Self {
            device,
            buffer: Vec::new(),
        }
    }
}

impl<D> Device<D>
where
    D: Read,
{
    /// Read in until device is exhausted
    fn update(&mut self) {
        self.device.read_to_end(&mut self.buffer).ok();
    }
}

impl<D> PacketReader for Device<D>
where
    D: Read,
{
    fn read(&mut self) -> Option<crate::packets::ApplicationPacket> {
        self.update();
        let mut taken = 0;
        let mut potential = None;

        for start in 0..self.buffer.len() {
            let slice = &self.buffer[start..self.buffer.len()];

            let res: Result<(ApplicationPacket, usize), DecodeError> =
                decode_from_slice(slice, standard());

            match res {
                Ok(info) => {
                    let (packet, read) = info;
                    potential = Some(packet);
                    taken = start + read;
                    break;
                }

                #[allow(unused_variables)]
                Err(DecodeError::UnexpectedEnd { additional }) => {
                    taken = start;
                    break;
                }

                _ => {}
            }
        }

        self.buffer.rotate_left(taken);

        potential
    }
}

impl<D> PacketWriter for Device<D>
where
    D: Write,
{
    fn write<T: Into<ApplicationPacket>>(
        &mut self,
        packet: T,
    ) -> Result<(), bincode::error::EncodeError> {
        if let Err(e) = encode_into_std_write(packet.into(), &mut self.device, standard()) {
            Err(e)
        } else {
            Ok(())
        }
    }
}
