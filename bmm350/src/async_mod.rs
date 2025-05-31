#![no_std]
// TI INA260 Current Sensor
use crate::{CHIP_ID, CMD};
#[cfg(feature = "defmt")]
use defmt::error;
use defmt::info;
#[cfg(feature = "async")]
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::i2c::I2c as AsyncI2c;

pub struct AsyncBMM350<I2C, Delay> {
    i2c: I2C,
    pub address: u8,
    delay: Delay,
}
#[cfg(feature = "async")]
impl<I2C: AsyncI2c, D> AsyncBMM350<I2C, D>
where
    I2C: AsyncI2c,
    D: DelayNs,
{
    pub fn new(i2c: I2C, address: u8, delay: D) -> Self {
        AsyncBMM350 {
            i2c,
            address,
            delay,
        }
    }
    pub async fn init(&mut self) -> Result<bool, I2C::Error> {
        let soft_reset_result = self.soft_reset().await;
        match soft_reset_result {
            Ok(soft_reset) => {
                if soft_reset {
                    info!("Soft Reset Successful");
                } else {
                    error!("Soft Reset Failed");
                }
            }
            Err(i2c_error) => {
                error!("BMM350 I2C Error");
                return Err(i2c_error);
            }
        }
        let chip_id = self.get_chip_id().await;
        match chip_id {
            Ok(chip_id) => {
                if chip_id == 0x33 {
                    info!("BMM350 Chip ID is correct");
                    Ok(true)
                } else {
                    error!("BMM350 Chip ID is incorrect: {}", chip_id);
                    Ok(false)
                }
            }
            Err(i2c_error) => {
                error!("BMM350 I2C Error");
                Err(i2c_error)
            }
        }
    }

    async fn write_register(&mut self, register: u8, data: u8) -> Result<(), I2C::Error> {
        self.i2c.write(self.address, &[register, data]).await
    }

    async fn read_register(&mut self, reg: u8) -> Result<[u8; 1], I2C::Error> {
        let mut buf = [0; 1];
        let result = self.i2c.write_read(self.address, &[reg], &mut buf).await;

        match result {
            Ok(_) => {
                Ok(buf)
            }
            Err(i2c_error) => {
                Err(i2c_error)
            }
        }
    }

    async fn get_chip_id(&mut self) -> Result<u8, I2C::Error> {
        let init_result = self.read_register(CHIP_ID.0).await;
        match init_result {
            Ok(chip_id) => {
                Ok(chip_id[0])
            }
            Err(i2c_error) => {
                Err(i2c_error)
            }
        }
    }

    async fn soft_reset(&mut self) -> Result<bool, I2C::Error> {
        let mut buf = [0; 2];
        let reset_cmd_result = self
            .i2c
            .write_read(self.address, &[CMD.0, 0xb6], &mut buf)
            .await;
        self.delay.delay_ns(1).await;
        match reset_cmd_result {
            Ok(reset_cmd_buf) => {
                if buf[0] == 0x00 {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(i2c_error) => {
                Err(i2c_error)
            }
        }
    }
}
