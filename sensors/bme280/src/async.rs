#![no_std]
use crate::{Register};
#[cfg(feature = "defmt")]
use defmt::error;
use defmt::info;
#[cfg(feature = "async")]
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::i2c::I2c as AsyncI2c;
use embedded_hal_async::i2c::{
    Error,
    ErrorKind,
    NoAcknowledgeSource
};

#[derive(Debug)]
pub enum Bme280Error<I2cError> {
    // I2C Bus error like nack or tiemout
    I2c(I2cError),
    // Did not match the expected BME280 ID (0x60).
    InvalidChipId(u8),
}
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
    pub async fn init(&mut self) -> Result<(), Bme280Error<I2C::Error>> {
        let mut chip_id = [0u8; 1];

        // Attempt to read the CHIP_ID register (0xD0).
        match self.i2c.write_read(self.address, &[0xD0], &mut chip_id).await {
            Ok(_) => {
                if chip_id[0] != 0x60 {
                    error!("BME280 connection failed: Invalid Chip ID detected. Expected 0x60, got {:#04X}", chip_id[0]);
                    
                    return Err(Bme280Error::InvalidChipId(chip_id[0]));
                }
                info!("BME280 detected successfully on I2C bus.");
            }
            Err(e) => {
        
        let kind = e.kind(); 
        
        match kind {
                ErrorKind::NoAcknowledge(source) => {
                    match source {
                        NoAcknowledgeSource::Address => {
                            error!("I2C Error: No ACK on Address {:#02X}. Is the sensor wired correctly?", self.address);
                        }
                        NoAcknowledgeSource::Data => {
                            error!("I2C Error: No ACK on Data. The sensor dropped the connection mid-stream.");
                        }
                        _ => error!("I2C Error: Unknown NoACK source."),
                    }
                }
                ErrorKind::Bus => {
                    error!("I2C Error: Bus Error. Check your pull-up resistors and wire lengths.");
                }
                ErrorKind::ArbitrationLoss => {
                    error!("I2C Error: Arbitration Loss. Is another controller on the bus?");
                }
                ErrorKind::Overrun => {
                    error!("I2C Error: Overrun. The hardware couldn't keep up with the clock speed.");
                }
                _ => {
                    error!("I2C Error: An unspecified or 'Other' error occurred: {:?}", kind);
                }
            }
            return Err(Bme280Error::I2c(e));
        }
        }

        //  Configuration Application
        self.i2c
            .write(self.address, &[0xF2, 0x01])
            .await
            .map_err(Bme280Error::I2c)?;

        // ctrl_meas (0xF4): temp and pressure oversampling x1, mode = normal
        self.i2c
            .write(self.address, &[0xF4, 0x27])
            .await
            .map_err(Bme280Error::I2c)?;

        // config (0xF5): standby time 1000ms, IIR filter off
        self.i2c
            .write(self.address, &[0xF5, 0xA0])
            .await
            .map_err(Bme280Error::I2c)?;

        self.delay.delay_ms(2).await;
        

        info!("BME280 initialization complete.");
        
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