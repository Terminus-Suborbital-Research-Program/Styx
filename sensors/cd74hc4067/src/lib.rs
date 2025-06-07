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

use embedded_hal::digital::{OutputPin};

// Types/Constants
pub enum Channel{
    Disable =   0b00001,
    Channel0 =  0b00000,
    Channel1 =  0b10000,
    Channel2 =  0b01000,
    Channel3 =  0b11000,
    Channel4 =  0b00100,
    Channel5 =  0b10100,
    Channel6 =  0b01100,
    Channel7 =  0b11100,
    Channel8 =  0b00010,
    Channel9 =  0b10010,
    Channel10 = 0b01010,
    Channel11 = 0b11010,
    Channel12 = 0b00110,
    Channel13 = 0b10110,
    Channel14 = 0b01110,
    Channel15 = 0b11110,
}

// Library
pub struct CD74HC4067<S0: OutputPin, S1: OutputPin, S2: OutputPin, S3: OutputPin, E: OutputPin>{
    s0: S0,
    s1: S1,
    s2: S2,
    s3: S3,
    enable: E // When high, disables all switches
}
impl<S0: OutputPin, S1: OutputPin, S2: OutputPin, S3: OutputPin, E: OutputPin> CD74HC4067<S0, S1, S2, S3, E>{
    pub fn new_enable(s0: S0, s1: S1, s2: S2, s3: S3, enable: E) -> Self {
        CD74HC4067 {
            s0,
            s1,
            s2,
            s3,
            enable,
        }
    }
    pub fn set_pin(&mut self, channel: &Channel){
        // Set the pins according to the channel
        match channel {
            Channel::Disable => {
                self.s0.set_low().ok();
                self.s1.set_low().ok();
                self.s2.set_low().ok();
                self.s3.set_low().ok();
                self.enable.set_high().ok();
            },
            Channel::Channel0 => {
                self.s0.set_low().ok();
                self.s1.set_low().ok();
                self.s2.set_low().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel1 => {
                self.s0.set_high().ok();
                self.s1.set_low().ok();
                self.s2.set_low().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel2 => {
                self.s0.set_low().ok();
                self.s1.set_high().ok();
                self.s2.set_low().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel3 => {
                self.s0.set_high().ok();
                self.s1.set_high().ok();
                self.s2.set_low().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel4 => {
                self.s0.set_low().ok();
                self.s1.set_low().ok();
                self.s2.set_high().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel5 => {
                self.s0.set_high().ok();
                self.s1.set_low().ok();
                self.s2.set_high().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel6 => {
                self.s0.set_low().ok();
                self.s1.set_high().ok();
                self.s2.set_high().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel7 => {
                self.s0.set_high().ok();
                self.s1.set_high().ok();
                self.s2.set_high().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel8 => {
                self.s0.set_low().ok();
                self.s1.set_low().ok();
                self.s2.set_low().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel9 => {
                self.s0.set_high().ok();
                self.s1.set_low().ok();
                self.s2.set_low().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel10 => {
                self.s0.set_low().ok();
                self.s1.set_high().ok();
                self.s2.set_low().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel11 => {
                self.s0.set_high().ok();
                self.s1.set_high().ok();
                self.s2.set_low().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel12 => {
                self.s0.set_low().ok();
                self.s1.set_low().ok();
                self.s2.set_high().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel13 => {
                self.s0.set_high().ok();
                self.s1.set_low().ok();
                self.s2.set_high().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel14 => {
                self.s0.set_low().ok();
                self.s1.set_high().ok();
                self.s2.set_high().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel15 => {
                self.s0.set_low().ok();
                self.s1.set_high().ok();
                self.s2.set_high().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            _ => {}
        }
    }
    pub async fn set_pin_async(&mut self, channel: Channel){
        // Set the pins according to the channel
        match channel {
            Channel::Disable => {
                self.s0.set_low().ok();
                self.s1.set_low().ok();
                self.s2.set_low().ok();
                self.s3.set_low().ok();
                self.enable.set_high().ok();
            },
            Channel::Channel0 => {
                self.s0.set_low().ok();
                self.s1.set_low().ok();
                self.s2.set_low().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel1 => {
                self.s0.set_high().ok();
                self.s1.set_low().ok();
                self.s2.set_low().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel2 => {
                self.s0.set_low().ok();
                self.s1.set_high().ok();
                self.s2.set_low().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel3 => {
                self.s0.set_high().ok();
                self.s1.set_high().ok();
                self.s2.set_low().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel4 => {
                self.s0.set_low().ok();
                self.s1.set_low().ok();
                self.s2.set_high().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel5 => {
                self.s0.set_high().ok();
                self.s1.set_low().ok();
                self.s2.set_high().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel6 => {
                self.s0.set_low().ok();
                self.s1.set_high().ok();
                self.s2.set_high().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel7 => {
                self.s0.set_high().ok();
                self.s1.set_high().ok();
                self.s2.set_high().ok();
                self.s3.set_low().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel8 => {
                self.s0.set_low().ok();
                self.s1.set_low().ok();
                self.s2.set_low().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel9 => {
                self.s0.set_high().ok();
                self.s1.set_low().ok();
                self.s2.set_low().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel10 => {
                self.s0.set_low().ok();
                self.s1.set_high().ok();
                self.s2.set_low().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel11 => {
                self.s0.set_high().ok();
                self.s1.set_high().ok();
                self.s2.set_low().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel12 => {
                self.s0.set_low().ok();
                self.s1.set_low().ok();
                self.s2.set_high().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel13 => {
                self.s0.set_high().ok();
                self.s1.set_low().ok();
                self.s2.set_high().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel14 => {
                self.s0.set_low().ok();
                self.s1.set_high().ok();
                self.s2.set_high().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            Channel::Channel15 => {
                self.s0.set_low().ok();
                self.s1.set_high().ok();
                self.s2.set_high().ok();
                self.s3.set_high().ok();
                self.enable.set_low().ok();
            },
            _ => {}
        }
    }
}
