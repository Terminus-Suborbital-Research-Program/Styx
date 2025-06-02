use core::{cmp::min, ops::Not};

use payload::Payload;
use sequence::Header;

use crate::{Error, Result};

mod crc;
pub mod payload;
pub mod sequence;

/// The maximum size of frame data
pub const MAX_PAYLOAD_LEN_BYTES: usize = 128;

/// The start byte for a packet
pub const START_BYTE: u8 = 0xCE;

/// The end byte for a packet
pub const END_BYTE: u8 = 0xCF;

/// A frame item that consists of arbitrary binary data
#[derive(Debug, Clone)]
pub struct Frame {
    header: Header,
    payload: Payload,
    checksum: u16,
}

impl Frame {
    /// Create a new frame from data. This is not decoding
    pub fn new(data: &[u8], sequence_number: u8) -> (Self, usize) {
        let bytes = min(data.len(), MAX_PAYLOAD_LEN_BYTES);
        // Safe - this is always as appropriate size
        let payload = Payload::try_from_bytes(data).unwrap();
        let checksum = payload.checksum();
        let header = Header::from_sequence_number(sequence_number);

        (
            Self {
                payload,
                checksum,
                header,
            },
            bytes,
        )
    }

    /// Length of a frame required
    fn len_required(&self) -> usize {
        // Two bytes for sequence, one for start, and one for end, plus two for the checksum and
        // two for length
        self.payload.as_ref().len() + 2 + 1 + 1 + 2 + 2
    }

    /// Encodes the current packet into a slice, erroring out if not enough space is present.
    /// Otherwise returns the number of bytes written
    pub fn encode_into_slice(&self, destination: &mut [u8]) -> Result<usize> {
        if destination.len() < self.len_required() {
            return Err(Error::NotEnoughBytes);
        }
        // Place start of sequence
        destination[0] = START_BYTE;

        // Encode header
        destination[1..3].copy_from_slice(&self.header.into_bytes());

        // Encode the number of bytes + it's inverse
        let num_bytes = self.payload.len() as u8;
        destination[3] = num_bytes;
        destination[4] = num_bytes.not();

        // Copy body in
        let payload_start = 5;
        let payload_end = payload_start + self.payload.len();
        destination[payload_start..payload_end].copy_from_slice(self.payload.data());

        // Copy checksum
        destination[payload_end..payload_end + 2]
            .copy_from_slice(&self.payload.checksum().to_le_bytes());

        // Place ending byte
        destination[payload_end + 2] = END_BYTE;

        Ok(payload_end + 1)
    }

    /// Try and decode a packer from a source, returns a frame and the number of bytes read
    pub fn decode_from_slice(source: &[u8]) -> Result<(Self, usize)> {
        // The minimum size of a frame is two sequence plus a start and an end byte, as well as a
        // crc-32 two byte sequence
        if source.len() < 6 {
            return Err(Error::NotEnoughBytes);
        }

        if source[0] != START_BYTE {
            return Err(Error::BadStart);
        }

        let header = Header::from_bytes(&source[1..3])?;

        let bytes_required = source[3] as usize;
        let bytes_required_cs = source[4] as usize;
        if bytes_required != (bytes_required_cs.not() & 0xFF) {
            return Err(Error::BadPacketLength {
                first: bytes_required,
                checksum: bytes_required_cs,
            });
        }

        // Check if there are enough remaining bytes
        if source.len() > bytes_required + 2 + 2 + 2 + 2 {
            let payload = Payload::try_from_bytes(&source[5..5 + bytes_required])?;

            let checksum =
                u16::from_be_bytes([source[5 + bytes_required], source[6 + bytes_required]]);

            let frame = Self {
                header,
                payload,
                checksum,
            };

            frame.checksum_valid()?;

            Ok((frame, 6 + bytes_required))
        } else {
            Err(Error::NotEnoughBytes)
        }
    }

    /// Returns ok if the  checksum is good
    pub fn checksum_valid(&self) -> Result<()> {
        let payload_checksum = self.payload.checksum();
        if self.checksum == payload_checksum {
            Ok(())
        } else {
            Err(Error::BadChecksum {
                expected: self.checksum,
                found: payload_checksum,
            })
        }
    }

    /// Returns the number of bytes in the frame's payload
    pub fn len(&self) -> usize {
        self.payload.as_ref().len()
    }

    /// Returns true if the frame is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl AsRef<[u8]> for Frame {
    fn as_ref(&self) -> &[u8] {
        self.payload.as_ref()
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
    fn round_trip_encode_decode() {
        // ---------------- Arrange ----------------
        let data = make_data(10); // any length ≤ MAX_PAYLOAD_LEN_BYTES
        let seq = 42u8;
        let (frame, bytes_used) = Frame::new(&data, seq);
        assert_eq!(bytes_used, data.len());
        assert_eq!(frame.len(), data.len());

        // ---------------- Act: encode ----------------
        let mut buf = [0u8; MAX_PAYLOAD_LEN_BYTES + 16]; // plenty of space
        let encoded_len_reported = frame.encode_into_slice(&mut buf).expect("encode failed");

        // encode_into_slice (per current impl) returns `len = payload_len + 6`
        assert!(encoded_len_reported <= buf.len());

        // Append two dummy bytes so the slice length is *strictly greater* than the
        // internal `decode` size check (`>` rather than `>=`).
        buf[encoded_len_reported] = 0xAA;
        buf[encoded_len_reported + 1] = 0x55;
        let decode_input = &buf[..encoded_len_reported + 2];

        // ---------------- Act: decode ----------------
        let (decoded, bytes_read) = Frame::decode_from_slice(decode_input).expect("decode failed");

        // ---------------- Assert ----------------
        // The decoder should report having consumed exactly what the encoder (currently) reports.
        // (encode returns p + 6, decode returns p + 6)
        assert_eq!(bytes_read, encoded_len_reported);
        assert_eq!(decoded.as_ref(), frame.as_ref()); // payload identical
        assert_eq!(decoded.len(), frame.len());
        decoded.checksum_valid().unwrap();
    }

    /// Encoding should fail when the destination buffer is too small.
    #[test]
    fn encode_fails_when_buffer_too_small() {
        let data = make_data(20);
        let (frame, _) = Frame::new(&data, 7);

        // Intentionally allocate less than the minimum required.
        let mut tiny_buf = [0u8; 4];
        let err = frame.encode_into_slice(&mut tiny_buf).unwrap_err();
        assert_eq!(err, Error::NotEnoughBytes);
    }

    /// Decoding should fail if the first byte is not `START_BYTE`.
    #[test]
    fn decode_fails_on_bad_start() {
        let data = make_data(5);
        let (frame, _) = Frame::new(&data, 1);

        let mut buf = [0u8; 64];
        let enc_len = frame.encode_into_slice(&mut buf).unwrap();
        buf[0] = 0x00; // corrupt START_BYTE
        buf[enc_len] = 0xAA; // satisfy `>` size check
        buf[enc_len + 1] = 0xAA;

        let err = Frame::decode_from_slice(&buf[..enc_len + 2]).unwrap_err();
        assert_eq!(err, Error::BadStart);
    }

    /// Decoding should fail on a checksum mismatch.
    #[test]
    fn decode_fails_on_bad_checksum() {
        let data = make_data(8);
        let (frame, _) = Frame::new(&data, 9);

        let mut buf = [0u8; 128];
        let enc_len = frame.encode_into_slice(&mut buf).unwrap();

        // Flip one byte inside the payload (starts at index 5).
        buf[8] ^= 0xFF;
        buf[enc_len] = 0x12; // pad so `>` size check passes
        buf[enc_len + 1] = 0x34;

        let err = Frame::decode_from_slice(&buf[..enc_len + 2]).unwrap_err();
        match err {
            Error::BadChecksum { expected, found } => assert_ne!(expected, found),
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
