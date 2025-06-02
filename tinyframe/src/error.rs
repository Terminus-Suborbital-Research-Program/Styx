/// Custom error type
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Bad sequence
    BadSequence([u8; 2]),
    /// Unexpected end of bytes
    NotEnoughBytes,
    /// Too many bytes were requested in payload
    BadPayloadSize(usize),
    /// Bad start byte
    BadStart,
    /// Bad packet length
    BadPacketLength { first: usize, checksum: usize },
    /// Bad checksum
    BadChecksum { expected: u16, found: u16 },
}

/// Custom result type
pub type Result<T> = core::result::Result<T, Error>;
