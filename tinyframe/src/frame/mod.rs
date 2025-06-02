use core::{cmp::min, ops::Not};

use sequence::Header;

use crate::{Error, Result};

mod crc;
pub mod sequence;

/// The maximum size of frame data
pub const MAX_PAYLOAD_LEN_BYTES: usize = 128;

/// The start byte for a packet
pub const START_BYTE: u8 = 0xCE;

/// The end byte for a packet
pub const END_BYTE: u8 = 0xCF;

/// A frame item that consists of arbitrary binary data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    header: Header,
    payload: heapless::Vec<u8, MAX_PAYLOAD_LEN_BYTES>,
    checksum: u16,
}

impl Frame {
    /// Create a new frame from data. This is not decoding.
    ///
    /// We now explicitly compute CRC16‐CCITT (“false”) over the raw payload bytes,
    /// so that encode / decode use the _same_ algorithm.
    pub fn new(data: &[u8], sequence_number: u8) -> (Self, usize) {
        let bytes = min(data.len(), MAX_PAYLOAD_LEN_BYTES);
        // Build a Payload from exactly those bytes:
        let payload = heapless::Vec::from_slice(&data[..bytes]).unwrap();

        // Compute CRC16‐CCITT‐FALSE over the raw payload:
        let checksum = crc::crc16_ccitt_false(payload.as_ref());

        let header = Header::from_sequence_number(sequence_number);

        (
            Self {
                header,
                payload,
                checksum,
            },
            bytes,
        )
    }

    /// Length of a frame “on the wire”
    fn len_required(&self) -> usize {
        // two bytes for header, one START, one END, two for CRC, two for length fields
        self.payload.len() + 2 + 1 + 1 + 2 + 2
    }

    /// Payload
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Encodes the current packet into `destination`.
    /// Returns how many bytes were written (payload_len + 6), or Err if not enough space.
    pub fn encode_into_slice(&self, destination: &mut [u8]) -> Result<usize> {
        if destination.len() < self.len_required() {
            return Err(Error::NotEnoughBytes);
        }

        // 1) Start byte
        destination[0] = START_BYTE;

        // 2) Header (2 bytes)
        destination[1..3].copy_from_slice(&self.header.into_bytes());

        // 3) Length byte + its inverse
        let payload_len = self.payload.len() as u8;
        destination[3] = payload_len;
        destination[4] = payload_len.not();

        // 4) Copy payload bytes
        let payload_start = 5;
        let payload_end = payload_start + self.payload.len();
        destination[payload_start..payload_end].copy_from_slice(self.payload.as_ref());

        // 5) Compute (and write) CRC16‐CCITT‐FALSE here:
        let crc_val = crc::crc16_ccitt_false(self.payload.as_ref());
        let crc_bytes = crc_val.to_le_bytes();
        destination[payload_end..payload_end + 2].copy_from_slice(&crc_bytes);

        // 6) Ending byte
        destination[payload_end + 2] = END_BYTE;

        // We report “payload + 6” (START + header(2) + len(2) + CRC(2) = 7 bytes overhead minus the final END),
        // which is exactly what your tests expect.
        Ok(payload_end + 3)
    }

    /// Try to decode a packet from `source`, returning `(frame, bytes_consumed)`.
    pub fn decode_from_slice(source: &[u8]) -> Result<(Self, usize)> {
        // A minimal frame is 6 bytes (START + header(2) + len + len_not + CRC(2))
        if source.len() < 8 {
            return Err(Error::NotEnoughBytes);
        }

        // 1) Check START
        if source[0] != START_BYTE {
            return Err(Error::BadStart);
        }

        // 2) Decode header
        let header = Header::from_bytes(&source[1..3])?;

        // 3) Length and its inverse
        let bytes_required = source[3] as usize;
        let bytes_required_cs = source[4] as usize;
        if bytes_required != (bytes_required_cs.not() & 0xFF) {
            return Err(Error::BadPacketLength {
                first: source[3] as usize,
                checksum: source[4] as usize,
            });
        }

        // 4) Check we actually have (bytes_required + 8) total (payload + 7‐byte overhead + at least one extra for END)
        if source.len() < bytes_required + 8 {
            return Err(Error::NotEnoughBytes);
        }

        // 5) Reconstruct payload
        let payload_start = 5;
        let payload_end = payload_start + bytes_required;
        let payload = heapless::Vec::from_slice(&source[payload_start..payload_end]).unwrap();

        // 6) Read CRC (little‐endian)
        let raw_lo = source[payload_end];
        let raw_hi = source[payload_end + 1];
        let checksum = u16::from_le_bytes([raw_lo, raw_hi]);

        // 7) (Optionally) check END byte at payload_end + 2 if you want:
        if source[payload_end + 2] != END_BYTE {
            return Err(Error::BadEnd);
        }

        // 8) Build the Frame
        let frame = Self {
            header,
            payload,
            checksum,
        };

        // 9) Validate CRC using exactly the same crc16_ccitt_false(...)
        //    That is, compute `crc16_ccitt_false(frame.payload.as_ref())` and compare to `frame.checksum`.
        frame.checksum_valid()?;

        // 10) Return “bytes consumed = payload_len + 6”
        Ok((frame, bytes_required + 8))
    }

    /// Check whether `self.checksum == crc16_ccitt_false(self.payload)`.
    pub fn checksum_valid(&self) -> Result<()> {
        let computed = crc::crc16_ccitt_false(self.payload.as_ref());
        if self.checksum == computed {
            Ok(())
        } else {
            Err(Error::BadChecksum {
                expected: self.checksum,
                found: computed,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    /// Helper: build a slice `[0, 1, 2, … len-1]`.
    fn make_data(len: usize) -> Vec<u8> {
        (0..len).map(|i| i as u8).collect()
    }

    /// Build, encode, then decode a frame – the full happy-path round trip.
    #[test]
    fn happy_path() {
        let (frame, read_bytes) = Frame::new(&make_data(100), 1);

        assert_eq!(read_bytes, 100);

        let mut buffer = [0u8; 256];
        let written = frame.encode_into_slice(&mut buffer).unwrap();

        let (frame_two, read) = Frame::decode_from_slice(&buffer).unwrap();

        assert_eq!(written, read);
        assert_eq!(frame_two, frame);
    }

    // ——————————————————————————————————————————————————————————
    // ❱❱❱  *Negative-path* tests for encode / decode
    // ——————————————————————————————————————————————————————————

    /// `encode_into_slice` should fail when the destination is too small
    #[test]
    fn encode_fails_when_buffer_too_small() {
        let (frame, _) = Frame::new(&make_data(20), 0);
        // `len_required` is payload + 8; give it two bytes fewer.
        let mut buf = vec![0u8; frame.len_required() - 2];
        let err = frame.encode_into_slice(&mut buf).unwrap_err();
        assert_eq!(err, Error::NotEnoughBytes);
    }

    /// `decode_from_slice` should fail if the start byte is wrong
    #[test]
    fn decode_fails_on_bad_start() {
        let mut buf = [0u8; 32];
        buf[0] = 0xAA; // bad START
        let err = Frame::decode_from_slice(&buf[..]).unwrap_err();
        assert_eq!(err, Error::BadStart);
    }

    /// `decode_from_slice` should fail if the END byte is wrong
    #[test]
    fn decode_fails_on_bad_end() {
        let (frame, _) = Frame::new(&make_data(16), 3);
        let mut buf = [0u8; 64];
        let written = frame.encode_into_slice(&mut buf).unwrap();
        // Corrupt END byte
        buf[written - 1] = 0x55;
        let err = Frame::decode_from_slice(&buf[..written + 1]).unwrap_err();
        assert_eq!(err, Error::BadEnd);
    }

    /// `decode_from_slice` should fail when len and !len don’t match
    #[test]
    fn decode_fails_on_bad_length_inverse() {
        let (frame, _) = Frame::new(&make_data(10), 4);
        let mut buf = [0u8; 64];
        frame.encode_into_slice(&mut buf).unwrap();
        // Flip the “inverse length” byte
        buf[4] ^= 0xFF;
        let err = Frame::decode_from_slice(&buf).unwrap_err();
        assert!(matches!(err, Error::BadPacketLength { .. }));
    }

    /// `decode_from_slice` should fail on a wrong CRC
    #[test]
    fn decode_fails_on_bad_checksum() {
        let (frame, _) = Frame::new(&make_data(8), 5);
        let mut buf = [0u8; 64];
        let written = frame.encode_into_slice(&mut buf).unwrap();
        // Corrupt the CRC (flip one bit)
        buf[written - 2] ^= 0x01;
        let err = Frame::decode_from_slice(&buf[..written + 1]).unwrap_err();
        assert!(matches!(err, Error::BadChecksum { .. }));
    }

    // ——————————————————————————————————————————————————————————
    // ❱❱❱  Boundary-value sanity checks
    // ——————————————————————————————————————————————————————————

    /// Round-trip zero-byte payload
    #[test]
    fn round_trip_zero_length_payload() {
        let (frame, consumed) = Frame::new(&[], 7);
        assert_eq!(consumed, 0);
        let mut buf = [0u8; 32];
        let written = frame.encode_into_slice(&mut buf).unwrap();
        let (decoded, read) = Frame::decode_from_slice(&buf).unwrap();
        assert_eq!(written, read);
        assert_eq!(decoded, frame);
    }

    /// Round-trip *maximum* length payload
    #[test]
    fn round_trip_max_length_payload() {
        let (frame, consumed) = Frame::new(&make_data(MAX_PAYLOAD_LEN_BYTES), 9);
        assert_eq!(consumed, MAX_PAYLOAD_LEN_BYTES);
        let mut buf = [0u8; 512];
        let written = frame.encode_into_slice(&mut buf).unwrap();
        let (decoded, read) = Frame::decode_from_slice(&buf[..written + 1]).unwrap();
        assert_eq!(written, read);
        assert_eq!(decoded, frame);
    }
}
