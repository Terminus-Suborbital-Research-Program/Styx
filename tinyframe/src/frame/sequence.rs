
use crate::{Error, Result};

/// A frame sequence
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Header {
    /// The start of a transmission. Implicitly known as frame zero
    Start,
    /// Sequence number within a transmission
    FrameNumber(usize),
    /// End of a transmission
    End,
}

impl Header {
    /// Convert into bytes
    pub fn into_bytes(&self) -> [u8; 2] {
        match self {
            Self::Start => [0x00, 0x00],
            Self::FrameNumber(num) => [0x01, *num as u8],
            Self::End => [0x02, 0x00],
        }
    }

    /// Try to read out of a slice. Will always read the top two bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 2 {
            Err(Error::NotEnoughBytes)
        } else {
            match [data[0], data[1]] {
                [0x00, 0x00] => Ok(Self::Start),

                [0x02, 0x00] => Ok(Self::End),

                [0x01, x] if 0 < x => Ok(Self::FrameNumber(x as usize)),

                _ => Err(Error::BadSequence([data[0], data[1]])),
            }
        }
    }

    /// Create from a sequence number
    pub fn from_sequence_number(number: u8) -> Self {
        if number == 0 {
            Self::Start
        } else {
            Self::FrameNumber(number as usize)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_bytes_start() {
        let seq = Header::Start;
        assert_eq!(seq.into_bytes(), [0x00, 0x00]);
    }

    #[test]
    fn into_bytes_frame_number_and_back() {
        // Choose a small payload index that is guaranteed <= MAX_PAYLOAD_LEN_BYTES
        let num = 5;
        assert!(
            num <= MAX_PAYLOAD_LEN_BYTES,
            "Test payload index exceeds MAX_PAYLOAD_LEN_BYTES"
        );
        let seq = Header::FrameNumber(num);
        let bytes = seq.into_bytes();
        assert_eq!(bytes, [0x01, num as u8]);

        // Round‐trip: from_bytes should reconstruct the same Sequence
        let restored = Header::from_bytes(&bytes).unwrap();
        assert_eq!(restored, Header::FrameNumber(num));
    }

    #[test]
    fn into_bytes_end() {
        let seq = Header::End;
        assert_eq!(seq.into_bytes(), [0x02, 0x00]);
    }

    #[test]
    fn from_bytes_too_short() {
        // Empty slice
        let err = Header::from_bytes(&[]).unwrap_err();
        assert_eq!(err, Error::NotEnoughBytes);

        // One‐byte slice
        let err = Header::from_bytes(&[0x01]).unwrap_err();
        assert_eq!(err, Error::NotEnoughBytes);
    }

    #[test]
    fn from_bytes_valid_start_and_end_even_with_extra_bytes() {
        // Even if slice is longer, only the first two bytes matter
        let data = [0x00, 0x00, 0xFF, 0xEE];
        let seq = Header::from_bytes(&data).unwrap();
        assert_eq!(seq, Header::Start);

        let data = [0x02, 0x00, 0x12];
        let seq = Header::from_bytes(&data).unwrap();
        assert_eq!(seq, Header::End);
    }

    #[test]
    fn from_bytes_bad_sequence_opcode() {
        // Opcode 0x03 is not defined
        let bad = [0x03, 0x00];
        let err = Header::from_bytes(&bad).unwrap_err();
        assert_eq!(err, Error::BadSequence([0x03, 0x00]));
    }
}
