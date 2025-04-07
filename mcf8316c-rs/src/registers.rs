use defmt::Format;
use sealed::Sealed;

use crate::{ControlWord, data_word_from_u32, data_word_to_u32};

mod sealed {
    pub trait Sealed {}
}

pub fn write_sequence(address: u16, value: u32) -> [u8; 7] {
    let control = ControlWord::new_write(address).to_bytes();
    let data = data_word_from_u32(value);

    [
        control[0], control[1], control[2], data[0], data[1], data[2], data[3],
    ]
}

/// Control register trait for easy access
pub trait Register: sealed::Sealed + From<u32> + Into<u32> + Copy {
    /// The associated address for the register
    const REGISTER_ADDRESS: u16;

    /// Generates the appropriate write sequence to modify the register to the current value
    fn write_transaction_bytes(&self) -> [u8; 7] {
        write_sequence(Self::REGISTER_ADDRESS, (*self).into())
    }

    /// Generates the appropriate read sequence for the register
    fn read_transaction_bytes() -> [u8; 3] {
        ControlWord::new_read(Self::REGISTER_ADDRESS).to_bytes()
    }

    /// Constructs self from a data word read
    fn from_data(data_word: [u8; 4]) -> Self {
        data_word_to_u32(data_word).into()
    }
}

/// ALGO_DEBUG Register. Used by us to control the speet of the motor
#[derive(Clone, Copy, Debug, Format)]
pub struct SpeedRegister {
    speed_percent: u8,
}

impl SpeedRegister {
    /// Convert to the 14-bit speed value for the register
    pub fn speed_percent_to_speed_ctrl(speed_percent: u8) -> u16 {
        // Max value for 14 bytes = 32767
        let speed = ((speed_percent as u32) * 0x7FFF) / 100;
        (speed & 0x7FFF) as u16 // Mask
    }

    /// Convert a u16 to the percent of maximum speed
    pub fn speed_ctrl_to_percent(speed_ctrl: u16) -> u8 {
        // Divide by max, multiply by one hundred
        (((speed_ctrl as u32) * 100 / 0x7FFF) & 0xFF) as u8
    }

    /// Create a new speed set
    pub fn new(speed_percent: u8) -> Self {
        let speed_percent = core::cmp::min(speed_percent, 100);
        SpeedRegister { speed_percent }
    }

    /// Change the speed
    pub fn set_speed_percentage(&mut self, speed: u8) {
        self.speed_percent = core::cmp::min(speed, 100);
    }

    /// Read the current target speed
    pub fn speed_percent(&self) -> u8 {
        self.speed_percent
    }
}

impl From<u32> for SpeedRegister {
    fn from(value: u32) -> Self {
        // Bytes 16-30 contain the 15-bit value
        let speed_ctrl = (value >> 16) & 0x7FFF;
        let speed_percent = SpeedRegister::speed_ctrl_to_percent(speed_ctrl as u16);
        SpeedRegister { speed_percent }
    }
}

impl From<SpeedRegister> for u32 {
    fn from(value: SpeedRegister) -> Self {
        let speed_ctrl = SpeedRegister::speed_percent_to_speed_ctrl(value.speed_percent) as u32;
        (speed_ctrl << 16) & 0x7FFF_0000
    }
}

impl Sealed for SpeedRegister {}
impl Register for SpeedRegister {
    const REGISTER_ADDRESS: u16 = 0xEC;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speed_percent_to_speed_ctrl() {
        assert_eq!(SpeedRegister::speed_percent_to_speed_ctrl(0), 0);
        assert_eq!(SpeedRegister::speed_percent_to_speed_ctrl(50), 16383); // 50% of 32767
        assert_eq!(SpeedRegister::speed_percent_to_speed_ctrl(100), 32767); // 100% of 32767
    }

    #[test]
    fn test_speed_ctrl_to_percent() {
        assert!((SpeedRegister::speed_ctrl_to_percent(0) as i32 - 0).abs() <= 1);
        assert!((SpeedRegister::speed_ctrl_to_percent(16383) as i32 - 50).abs() <= 1); // 50% of 32767
        assert!((SpeedRegister::speed_ctrl_to_percent(32767) as i32 - 100).abs() <= 1); // 100% of 32767
    }

    #[test]
    fn test_speed_conversion_equivalence() {
        // Assert within 1% of either direction
        let speed_percent = 42;
        let speed_ctrl = SpeedRegister::speed_percent_to_speed_ctrl(speed_percent);
        let converted_speed_percent = SpeedRegister::speed_ctrl_to_percent(speed_ctrl);
        assert!((speed_percent as i32 - converted_speed_percent as i32).abs() <= 1);
    }
}
