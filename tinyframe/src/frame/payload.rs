use crate::{Error, Result};

use super::{MAX_PAYLOAD_LEN_BYTES, crc::crc16_ccitt_false};
use heapless::Vec;

#[derive(Debug, Clone)]
pub struct Payload {
    data: Vec<u8, MAX_PAYLOAD_LEN_BYTES>,
}

impl AsRef<[u8]> for Payload {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

impl AsMut<[u8]> for Payload {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

impl Payload {
    /// Create a new payload
    pub fn new(data: Vec<u8, MAX_PAYLOAD_LEN_BYTES>) -> Self {
        Self { data }
    }

    /// Try and get a payload from a section of bytes. Will fail if not enough are present
    pub fn try_from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() > MAX_PAYLOAD_LEN_BYTES {
            Err(Error::BadPayloadSize(data.len()))
        } else {
            Ok(Self::new(Vec::from_slice(data).unwrap()))
        }
    }

    /// Length of self
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// If empty returns true
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Reference to data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Checksum of data
    pub fn checksum(&self) -> u16 {
        crc16_ccitt_false(&self.data)
    }
}
