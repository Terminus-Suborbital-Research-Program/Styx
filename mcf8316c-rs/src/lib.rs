// Lock no_std behind test not enabled
#![cfg_attr(not(test), no_std)]

pub(crate) mod control;
pub(crate) mod data;

/// Module for common registers
pub mod registers;

pub use control::*;
pub use data::*;
