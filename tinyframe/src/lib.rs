#![cfg_attr(not(test), no_std)]

mod bincode;
mod crc;
pub mod reader;
pub mod writer;

pub mod packets {
    /// Valid start of frame bit
    pub const SOF: u8 = 0x01;

    /// Max length per packet
    pub const MAX_PACKET_LEN: usize = 128;
    use core::ops::Not;

    use defmt::Format;

    use crate::crc::crc16_ccitt_false;

    #[derive(Clone, Copy, Debug, Format)]
    pub struct TinyFrame {
        length: usize,
        /// Maximum of 256 bits
        data: [u8; MAX_PACKET_LEN],
        /// Checksum of the data
        checksum: u16,
    }

    #[derive(Clone, Copy, Debug, Format)]
    pub enum FrameError {
        /// Bad start condition
        BadStart,
        /// Bad header (length wrong or checksum failure)
        BadHeader { first: u8, checksum: u8 },
        /// Not enough bytes in the buffer
        UnexpectedEOF,
        /// Bad CRC value
        BadCRC { got: u16, expected: u16 },
    }

    impl TinyFrame {
        /// Checksum of contained data
        fn checksum(&self) -> u16 {
            crc16_ccitt_false(&self.data)
        }

        /// Length of the frame
        pub fn frame_length(&self) -> usize {
            self.length
        }

        /// Frame data
        pub fn data(&self) -> &[u8] {
            &self.data[0..self.length]
        }

        /// Checks validity of packet
        fn check_validity(&self) -> Result<(), FrameError> {
            let calculated = self.checksum();
            if self.checksum == calculated {
                Ok(())
            } else {
                Err(FrameError::BadCRC {
                    expected: self.checksum,
                    got: calculated,
                })
            }
        }

        /// Takes a data source, and encodes it into packet, returning the packet and the number of
        /// bytes read from the buffer. This is a low level function not intended for consumption
        pub(crate) fn create_from_slice(bytes: &[u8]) -> (Self, usize) {
            let length = core::cmp::min(MAX_PACKET_LEN, bytes.len());

            let slice = &bytes[0..length];
            let mut data = [0u8; MAX_PACKET_LEN];
            data[0..length].copy_from_slice(slice);
            let checksum = crc16_ccitt_false(&data);

            (
                Self {
                    length,
                    data,
                    checksum,
                },
                length,
            )
        }

        /// Encodes this packet into a slice. Returns an error if the slice is not large enough,
        /// otherwise the number of bytes written
        pub fn encode_into_slice(&self, destination: &mut [u8]) -> Result<usize, FrameError> {
            // We need our length + 1 start, 2 length, and two checksum bytes
            let required_length = self.length + 5;
            if destination.len() < required_length {
                return Err(FrameError::UnexpectedEOF);
            }

            destination[0] = SOF;
            destination[1] = required_length as u8;
            destination[2] = required_length.not() as u8; // Checksum it
            destination[3..self.length + 3].copy_from_slice(&self.data[0..self.length]);

            let crc_start = self.length + 3;
            let crc_end = crc_start + 2;
            destination[crc_start..crc_end].copy_from_slice(&self.checksum().to_le_bytes());

            Ok(required_length)
        }

        /// Attempts to decode a packet from the buffer, returning itself and the number of bytes
        /// read from the source if successful
        pub fn decode_from_slice(bytes: &[u8]) -> Result<(Self, usize), FrameError> {
            if bytes.len() < 3 {
                return Err(FrameError::UnexpectedEOF); // Not enough bytes to read
            }

            if bytes[0] != 0x01 {
                // Not a start byte
                return Err(FrameError::BadStart);
            }

            if bytes[1] > MAX_PACKET_LEN as u8 {
                return Err(FrameError::BadStart);
            }

            if bytes[1] != bytes[2].not() {
                return Err(FrameError::BadHeader {
                    first: bytes[1],
                    checksum: bytes[2],
                });
            }

            let needed_len = bytes[1] as usize;

            // Check if enough bytes
            if bytes.len() < needed_len {
                return Err(FrameError::UnexpectedEOF);
            }

            let data_len = needed_len - 5;
            let mut data = [0u8; MAX_PACKET_LEN];
            data[0..data_len].copy_from_slice(&bytes[3..data_len + 3]);

            let crc_bytes = [bytes[data_len + 3], bytes[data_len + 4]];
            let crc = u16::from_le_bytes(crc_bytes);

            let packet = Self {
                length: data_len,
                data,
                checksum: crc,
            };

            packet.check_validity()?;

            Ok((packet, needed_len))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Helper that produces a deterministic byte pattern `[0, 1, 2, …]`
        fn sample_payload(len: usize) -> Vec<u8> {
            (0..len).map(|i| i as u8).collect()
        }

        #[test]
        fn roundtrip_encode_decode() {
            let payload = sample_payload(32);

            // Build a frame from raw data
            let (frame, consumed) = TinyFrame::create_from_slice(&payload);
            assert_eq!(consumed, payload.len());

            // Encode into a buffer large enough for the packet
            let mut buf = [0u8; MAX_PACKET_LEN + 5];
            let written = frame.encode_into_slice(&mut buf).unwrap();
            assert_eq!(buf[0], SOF, "SOF wrong!");
            assert_eq!(written, payload.len() + 5);

            // Decode back and verify equality
            let (decoded, read) = TinyFrame::decode_from_slice(&buf[..written]).unwrap();
            assert_eq!(read, written);
            assert_eq!(decoded.length, payload.len());
            assert_eq!(&decoded.data[..decoded.length], &payload[..]);
        }

        #[test]
        fn encode_into_too_small_slice_returns_error() {
            let payload = sample_payload(16);
            let (frame, _) = TinyFrame::create_from_slice(&payload);

            let mut small_buf = [0u8; 10]; // deliberately undersized
            matches!(
                frame.encode_into_slice(&mut small_buf),
                Err(FrameError::UnexpectedEOF)
            );
        }

        #[test]
        fn decode_rejects_bad_start() {
            let payload = sample_payload(8);
            let (frame, _) = TinyFrame::create_from_slice(&payload);

            let mut buf = [0u8; MAX_PACKET_LEN + 5];
            let written = frame.encode_into_slice(&mut buf).unwrap();
            buf[0] = 0xAA; // corrupt the SOF

            matches!(
                TinyFrame::decode_from_slice(&buf[..written]),
                Err(FrameError::BadStart)
            );
        }

        #[test]
        #[should_panic]
        fn decode_rejects_bad_header() {
            // 0x01 start, declared length 10, third byte *not* the complement of 10
            let bogus_header = [SOF, 10u8, 10u8];
            let _ = TinyFrame::decode_from_slice(&bogus_header).unwrap();
        }

        #[test]
        fn decode_rejects_unexpected_eof() {
            // Valid header for a 12-byte packet, but provide only the header itself
            let header_only = [SOF, 12u8, (!12u8)];
            matches!(
                TinyFrame::decode_from_slice(&header_only),
                Err(FrameError::UnexpectedEOF)
            );
        }

        #[test]
        fn decode_rejects_bad_crc() {
            let payload = sample_payload(20);
            let (frame, _) = TinyFrame::create_from_slice(&payload);

            // Encode
            let mut buf = [0u8; MAX_PACKET_LEN + 5];
            let written = frame.encode_into_slice(&mut buf).unwrap();

            // Flip one data byte to invalidate the CRC
            buf[5] ^= 0xFF;

            matches!(
                TinyFrame::decode_from_slice(&buf[..written]),
                Err(FrameError::BadCRC { .. })
            );
        }
    }
}
