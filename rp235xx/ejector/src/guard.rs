#![warn(missing_docs)]

use embedded_hal::i2c::I2c;

const SENSOR_ADDRESS: u8 = 0x60;
const PART_ID_REG: u8 = 0x00;
const SEQ_ID_REG: u8 = 0x02;

const PART_ID: u8 = 0x45;

const SEQ_ID: u8 = 0x08;

/// Sanity checks our solor luminocity sensor
#[allow(dead_code)] // This sensor probably dead
pub fn si1145_sanity_check<I: I2c>(device: &mut I) -> Result<(), I::Error> {
    let mut buf = [0u8; 1];
    device.write_read(SENSOR_ADDRESS, &[PART_ID_REG], &mut buf)?;

    #[cfg(debug_assertions)]
    assert_eq!(buf[0], PART_ID, "PART ID Mismatch!");

    device.write_read(SENSOR_ADDRESS, &[SEQ_ID_REG], &mut buf)?;

    #[cfg(debug_assertions)]
    assert_eq!(buf[0], SEQ_ID, "Sequence ID mismatch!");

    Ok(())
}
