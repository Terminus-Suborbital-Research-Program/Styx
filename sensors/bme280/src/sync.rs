use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;

pub struct BME280<I2C, Delay> {
    i2c: I2C,
    pub address: u8,
    delay: Delay,
}

impl<I2C, D> BME280<I2C, D>
where
    I2C: I2c,
    D: DelayNs,
{
    pub fn new(i2c: I2C, address: u8, delay: D) -> Self {
        Self {
            i2c,
            address,
            delay,
        }
    }

    pub fn init(&mut self) -> Result<(), I2C::Error> {
        self.i2c.write(self.address, &[0xF2, 0x01])?;
        self.i2c.write(self.address, &[0xF4, 0x27])?;
        self.i2c.write(self.address, &[0xF5, 0xA0])?;

        self.delay.delay_ms(2);
        Ok(())
    }

    pub fn sample(&mut self) -> Result<([u8; 8], u32, u32, u16), I2C::Error> {
        let register = [0xF7];
        let mut buffer = [0u8; 8];

        self.i2c.write_read(self.address, &register, &mut buffer)?;

        let raw_pressure: u32 =
            ((buffer[0] as u32) << 12) | ((buffer[1] as u32) << 4) | ((buffer[2] as u32) >> 4);
        let raw_temp: u32 =
            ((buffer[3] as u32) << 12) | ((buffer[4] as u32) << 4) | ((buffer[5] as u32) >> 4);
        let raw_hum: u16 = ((buffer[6] as u16) << 8) | (buffer[7] as u16);

        Ok((buffer, raw_pressure, raw_temp, raw_hum))
    }
}
