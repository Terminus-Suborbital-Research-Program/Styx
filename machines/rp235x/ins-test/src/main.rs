#![no_std]
#![no_main]

#[cfg(feature = "wifi")]
mod main_wifi;

#[cfg(not(feature = "wifi"))]
mod main_blocking;
