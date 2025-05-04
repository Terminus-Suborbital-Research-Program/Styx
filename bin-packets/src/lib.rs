#![cfg_attr(not(any(test, feature = "std")), no_std)] // If we're not testing, don't link the standard library

pub mod commands;
pub mod data;
pub mod device;
pub mod devices;
pub mod packets;
pub mod phases;
pub mod time;
