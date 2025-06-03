#![no_std]
// TI INA260 Current Sensor
use crate::{Register};
#[cfg(feature = "defmt")]
use defmt::error;
use defmt::info;
#[cfg(feature = "async")]
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::i2c::I2c as AsyncI2c;

use crate::*;

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

    /// Return an error if it cannot communicate with the sensor.
    pub async fn status(&mut self) -> Result<Status, I2C::Error> {
        let status = self.read_u8(STATUS).await?.into();
        Ok(status)
    }

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        self.write_u8(SOFT_RESET, CMD_SOFT_RESET)
            .await?;
        self.delay.delay_ms(10).await;

        while self.status().await?.is_calibrating() {
            self.delay.delay_ms(10).await;
        }

        self.read_calibration_coefficients().await?;

        let configuration = Configuration::default();
        self.set_sampling_configuration(configuration).await?;

        self.delay.delay_ms(2).await;
        Ok(())
    }

    /// Read calibration coefficients from sensor
    async fn read_calibration_coefficients(&mut self) -> Result<(), I2C::Error> {
        let buffer: [u8; 1] = [calibration::FIRST_REGISTER];

        let mut out: [u8; calibration::TOTAL_LENGTH] = [0; calibration::TOTAL_LENGTH];
        self.i2c
            .write_read(
                self.address,
                &buffer,
                &mut out[0..calibration::FIRST_LENGTH],
            )
            .await?;

        let buffer: [u8; 1] = [calibration::SECOND_REGISTER];
        self.i2c
            .write_read(
                self.address,
                &buffer,
                &mut out[calibration::FIRST_LENGTH..calibration::TOTAL_LENGTH],
            )
            .await?;

        self.coefficients = (&out).into();

        Ok(())
    }


    pub async fn sample(&mut self)-> ([u8; 8], u16, u16, u16){
        let register = [0xF7]; // start of pressure/temp/humidity registers
        let mut buffer = [0u8; 8]; // pressure[3] + temp[3] + humidity[2]

        self.i2c.write_read(self.address, &register, &mut buffer).await;
        let raw_pressure = self.read_u16(PRESSURE).await.unwrap();
        let raw_humidity = self.read_u16(HUMID).await.unwrap();
        let raw_temperature = self.read_u16(TEMP).await.unwrap();
        (buffer, raw_pressure, raw_temperature, raw_humidity)
    }

/// Write an unsigned byte to an I²C register
    async fn write_u8(&mut self, register: u8, value: u8) -> Result<(), I2C::Error> {
        let buffer: [u8; 2] = [register, value];
        self.i2c.write(self.address, &buffer).await?;
        Ok(())
    }

    /// Write an unsigned byte from an I²C register
    async fn read_u8(&mut self, register: u8) -> Result<u8, I2C::Error> {
        let buffer: [u8; 1] = [register];
        let mut output_buffer: [u8; 1] = [0];
        self.i2c
            .write_read(self.address, &buffer, &mut output_buffer)
            .await?;
        Ok(output_buffer[0])
    }

    /// Write two unsigned bytes to an I²C register
    async fn read_u16(&mut self, register: u8) -> Result<u16, I2C::Error> {
        let buffer: [u8; 1] = [register];
        let mut output_buffer: [u8; 2] = [0, 0];
        self.i2c
            .write_read(self.address, &buffer, &mut output_buffer)
            .await?;
        Ok(u16::from(output_buffer[0]) << 8 | u16::from(output_buffer[1]))
    }

    /// Write three unsigned bytes to an I²C register
    async fn read_u24(&mut self, register: u8) -> Result<u32, I2C::Error> {
        let buffer: [u8; 1] = [register];
        let mut output_buffer: [u8; 3] = [0, 0, 0];
        self.i2c
            .write_read(self.address, &buffer, &mut output_buffer)
            .await?;
        Ok(u32::from(output_buffer[0]) << 12
            | u32::from(output_buffer[1]) << 4
            | u32::from(output_buffer[2]) >> 4)
    }
}