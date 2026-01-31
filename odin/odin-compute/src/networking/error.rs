use thiserror::Error;

// A wrapper around networking error types for the network socket trait
// so that I can work with multiple socket types in the same way.
#[derive(Error, Debug)]
pub enum IOError {
    #[error("Error sending packet to Jupiter")]
    RadioSendError,
    #[error("Error sending packet to Process Thread")]
    PacketChannelSendError,
}