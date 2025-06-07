#![no_std]
// Library
#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "sync")]
pub use sync::*;
#[cfg(feature = "async")]
pub mod r#async;
#[cfg(feature = "async")]
pub use r#async::*;

// Types/Constants
pub type Address = u8;
pub type Reset = u8;
pub type Register = (Address, Reset);
