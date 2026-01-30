
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SignalError {
    #[error("Buffer Overflow {0}")]
    PacketBufferOverflow(usize),
    #[error("Error Reading SDR Stream, sample length: {0} ")]
    StreamReadError(usize),
}