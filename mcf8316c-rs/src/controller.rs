use embedded_hal_async::i2c::I2c;

use crate::registers::{Register, SpeedRegister};

/// Motor control module for the MCF8316c. CANNOT BE USED ACROSS THREADS
pub struct MotorController<I> {
    address: u8,
    i2c: I,
}

impl<I: I2c> MotorController<I> {
    /// Create a new MotorController instance
    pub fn new(address: u8, i2c: I) -> Self {
        MotorController { address, i2c }
    }

    /// Get the address of the MotorController
    pub fn address(&self) -> u8 {
        self.address
    }

    /// Get the inner I2C device as a ref
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.i2c
    }

    /// Writes a register to the device
    pub async fn write_register<R: Register>(&mut self, register: R) -> Result<(), I::Error> {
        let bytes = register.write_transaction_bytes();
        self.i2c.write(self.address, &bytes).await
    }

    /// Reads a register from the device
    pub async fn read_register<R: Register>(&mut self) -> Result<R, I::Error> {
        let write_buffer = R::read_transaction_bytes();
        let mut read_buffer = [0u8; 4];

        match self
            .i2c
            .write_read(self.address, &write_buffer, &mut read_buffer)
            .await
        {
            Ok(_) => Ok(R::from_data(read_buffer)),
            Err(e) => Err(e),
        }
    }

    /// Get speed percent
    pub async fn get_speed_percent(&mut self) -> Result<u8, I::Error> {
        let register = self.read_register::<SpeedRegister>().await?;
        Ok(register.speed_percent())
    }

    /// Set speed percent
    pub async fn set_speed_percent(&mut self, percent: u8) -> Result<u32, I::Error> {
        let register = SpeedRegister::new(percent);
        match self.write_register(register).await {
            Ok(_) => Ok(register.into()),
            Err(e) => Err(e),
        }
    }
}
