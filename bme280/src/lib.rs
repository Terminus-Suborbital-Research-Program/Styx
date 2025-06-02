#![no_std]
// Library
#[cfg(feature = "sync")]
pub mod sync_mod;
#[cfg(feature = "sync")]
pub use sync_mod::*;
#[cfg(feature = "async")]
pub mod async_mod;
#[cfg(feature = "async")]
pub use async_mod::*;

// Types/Constants
pub type Address = u8;
pub type Reset = u8;
pub type Register = (Address, Reset);
