#![no_std]
use crate::{Register};
#[cfg(feature = "defmt")]
use defmt::error;
use defmt::info;
#[cfg(feature = "async")]
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::i2c::I2c as AsyncI2c;

pub struct AsyncBME280<I2C, Delay> {
    i2c: I2C,
    pub address: u8,
    delay: Delay,
}
#[cfg(feature = "async")]
impl<I2C: AsyncI2c, D> AsyncBME280<I2C, D>
where
    I2C: AsyncI2c,
    D: DelayNs,
{
    pub fn new(i2c: I2C, address: u8, delay: D) -> Self {
        AsyncBME280 {
            i2c,
            address,
            delay,
        }
    }
    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        self.i2c.write(self.address, &[0xF2, 0x01]).await?;
    
        // ctrl_meas (0xF4): temp and pressure oversampling x1, mode = normal
        self.i2c.write(self.address, &[0xF4, 0x27]).await?;

        // config (0xF5): standby time 1000ms, IIR filter off
        self.i2c.write(self.address, &[0xF5, 0xA0]).await?;

        self.delay.delay_ms(2).await;
        Ok(())
    }

    pub async fn sample(&mut self)-> ([u8; 8], u32, u32, u16){
        let register = [0xF7]; // start of pressure/temp/humidity registers
        let mut buffer = [0u8; 8]; // pressure[3] + temp[3] + humidity[2]

        self.i2c.write_read(self.address, &register, &mut buffer).await;

        // Parse raw readings from MSB → LSB
        let raw_pressure: u32 =
            ((buffer[0] as u32) << 12) | ((buffer[1] as u32) << 4) | ((buffer[2] as u32) >> 4);
        let raw_temp: u32 =
            ((buffer[3] as u32) << 12) | ((buffer[4] as u32) << 4) | ((buffer[5] as u32) >> 4);
        let raw_hum: u16 = ((buffer[6] as u16) << 8) | (buffer[7] as u16);

        (buffer, raw_pressure, raw_temp, raw_hum)
    }
}