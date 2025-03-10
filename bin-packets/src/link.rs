use bincode::{config::standard, encode_into_slice, Decode, Encode};
use defmt::Format;

use crate::{ApplicationPacket, DeviceIdentifier};

#[derive(Debug, Clone, Copy, Encode, Decode, Format)]
pub struct LinkPacket {
    pub from: DeviceIdentifier,
    pub to: DeviceIdentifier,
    pub payload: ApplicationPacket,
    checksum: u8,
}

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq, Format)]
pub struct ChecksumError {
    pub expected: u8,
    pub actual: u8,
}

impl LinkPacket {
    pub fn calculate_checksum(&self) -> u8 {
        let mut buffer = [0u8; 256];
        let mut clone = self.clone();
        clone.checksum = 0;
        let written = encode_into_slice(&clone, &mut buffer, standard()).unwrap();
        crc_32(&buffer[..written]) as u8
    }

    pub fn set_checksum(&mut self) {
        self.checksum = self.calculate_checksum();
    }

    pub fn new(from: DeviceIdentifier, to: DeviceIdentifier, payload: ApplicationPacket) -> Self {
        let mut packet = Self {
            from,
            to,
            payload,
            checksum: 0,
        };
        packet.set_checksum();
        packet
    }

    pub fn verify_checksum(&self) -> Result<(), ChecksumError> {
        let expected = self.calculate_checksum();
        if expected == self.checksum {
            Ok(())
        } else {
            Err(ChecksumError {
                expected,
                actual: self.checksum,
            })
        }
    }
}

fn crc_32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = -(crc as i32 & 1);
            crc = (crc >> 1) ^ (0xEDB88320 & mask as u32);
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_packet() {
        let packet = LinkPacket::new(
            DeviceIdentifier::Jupiter,
            DeviceIdentifier::Icarus,
            ApplicationPacket::Heartbeat {
                device: DeviceIdentifier::Jupiter,
                timestamp: 0,
                sequence_number: 0,
            },
        );
        assert_eq!(packet.verify_checksum(), Ok(()));
    }

    #[test]
    #[should_panic]
    fn test_link_packet_checksum_error() {
        let mut packet = LinkPacket::new(
            DeviceIdentifier::Jupiter,
            DeviceIdentifier::Icarus,
            ApplicationPacket::Heartbeat {
                device: DeviceIdentifier::Jupiter,
                timestamp: 0,
                sequence_number: 0,
            },
        );
        packet.checksum = 0;
        packet.verify_checksum().unwrap();
    }
}
