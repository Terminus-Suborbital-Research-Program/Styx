use embedded_hal::i2c::*;

/// Motor control structure
pub struct Motor<I> {
    address: u8,
    i2c: I,
}
