#![no_std]
// TI INA260 Current Sensor
#[cfg(feature = "defmt")]
use defmt::error;
use defmt::info;
#[cfg(feature = "async")]
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::i2c::{I2c as AsyncI2c, Error};
use crate::{Register, CHIP_ID};
use cast::{i32, u16, u32};

pub struct AsyncBMI323<I2C, Delay>{
    i2c: I2C,
    pub address: u8,
    delay: Delay,
}
#[cfg(feature = "async")]
impl<I2C: AsyncI2c, D> AsyncBMI323<I2C, D>
where
    I2C: AsyncI2c,
    D: DelayNs,{
    

        /// Create a new INA260 instance
        ///
        /// # Arguments
        ///
        /// * `i2c` - The I2C peripheral to use
        /// * `address` - The I2C address of the INA260
        pub fn new(i2c: I2C, address: u8, delay: D) -> Self {
            AsyncBMI323 {
                i2c,
                address,
                delay,
            }
        }
    
        pub async fn init(&mut self) -> Result<(), I2C::Error> {
            let chip_id = self.get_chip_id().await;
            match chip_id{
                Ok(id)=>{
                    if id != 43_u16{
                        error!("Chip ID does not match manufacturer chip ID. ID: {}", id);
                    }
                    return Ok(())
                }
                Err(i2c_error)=>{
                    return Err(i2c_error);
                }
            }
        }
    
        async fn write_register(&mut self, register: u8, data: u16) -> Result<(), I2C::Error> {
            self.i2c
                .write(
                    self.address,
                    &[register, (data >> 8) as u8, (data & 255) as u8],
                )
                .await
        }
    
        async fn read_register(&mut self, reg: u8) -> Result<[u8; 4], I2C::Error> {
            let mut buf = [0; 4];
            let result = self.i2c
                .write_read(self.address, &[reg], &mut buf)
                .await;
            match result{
                Ok(_)=>{
                    return Ok(buf);
                }
                Err(i2c_error)=>{
                    return Err(i2c_error);
                }
            }
            Ok(buf)
        }

        async fn get_chip_id(&mut self)->Result<u16, I2C::Error>{
            let init_result = self.read_register(CHIP_ID.0).await;
            match init_result{
                Ok(chip_id)=>{
                    let value = (u16::from(chip_id[0]) << 8 | u16::from(chip_id[1]) << 8);
                    info!("Chip ID: {}", value);
                    return Ok(value);
                }
                Err(i2c_error)=>{
                    info!("I2C Error: {}", i2c_error.kind());
                    return Err(i2c_error);
                }
            }
        }
}
