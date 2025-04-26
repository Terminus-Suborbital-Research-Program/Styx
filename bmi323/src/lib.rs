#![no_std]
#![cfg_attr(not(feature = "async"), deny(unstable_features))]

const PRIMARY_ADDRESS: u8 = 0x68;
const SECONDARY_ADDRESS: u8 = 0x69;
use defmt::Format;

// TI BMI323 Current Sensor
#[cfg(feature = "defmt")]
use defmt::info;
#[cfg(feature = "sync")]
use embedded_hal::i2c::{self, ErrorType, I2c, Operation, SevenBitAddress, TenBitAddress};
#[cfg(feature = "async")]
use embedded_hal_async::delay::DelayNs;
#[cfg(feature = "async")]
use embedded_hal_async::i2c::I2c as AsyncI2c;

// #[cfg(feature = "sync")]
// pub struct BMI323<I2C> {
//     i2c: I2C,
//     address: u8,
//     _marker: core::marker::PhantomData<I2C>,
//     state: u16,
// }
// #[cfg(feature = "sync")]
// impl<I2C: I2c> BMI323<I2C> {
//     /// Create a new BMI323 instance
//     ///
//     /// # Arguments
//     ///
//     /// * `i2c` - The I2C peripheral to use
//     /// * `address` - The I2C address of the BMI323
//     pub fn new(i2c: I2C, address: u8) -> Self {
//         BMI323 {}
//     }

//     pub fn init(&mut self) -> Result<(), I2C::Error> {}

//     pub fn write_register(&mut self, register: Register, data: u8) -> Result<(), I2C::Error> {}

//     pub fn read_register(&mut self, reg: Register) -> Result<u16, I2C::Error> {}
// }

#[derive(Debug, Format)]
pub struct AsyncBMI323<I2C, Delay> {
    i2c: I2C,
    address: u8,
    _marker: core::marker::PhantomData<I2C>,
    delay: Delay,
}
impl<I2C: AsyncI2c, D> AsyncBMI323<I2C, D>
where
    I2C: AsyncI2c,
    D: DelayNs,
{
    /// Create a new BMI323 instance
    ///
    /// # Arguments
    ///
    /// * `i2c` - The I2C peripheral to use
    /// * `address` - The I2C address of the BMI323
    pub fn new_primary(i2c: I2C, delay: D) -> Self {
        AsyncBMI323 {
            i2c,
            address: PRIMARY_ADDRESS,
            delay,
            _marker: core::marker::PhantomData,
        }
    }
    pub fn new_secondary(i2c: I2C, delay: D) -> Self {
        AsyncBMI323 {
            i2c,
            address: SECONDARY_ADDRESS,
            delay,
            _marker: core::marker::PhantomData,
        }
    }

    async fn write_register(&mut self, register: Register, data: u8) -> Result<(), I2C::Error> {
        self.i2c.write(self.address, &[register.addr(), data]).await
    }

    async fn read_register_8(&mut self, reg: Register) -> Result<[u8; 1], I2C::Error> {
        let mut buf = [0; 1];
        let result = self
            .i2c
            .write_read(self.address, &[reg.addr()], &mut buf)
            .await;
        match result {
            Ok(_) => {
                return Ok(buf);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    async fn read_register_16(&mut self, reg: Register) -> Result<[u8; 2], I2C::Error> {
        let mut buf = [0; 2];
        let result = self
            .i2c
            .write_read(self.address, &[reg.addr()], &mut buf)
            .await;
        match result {
            Ok(_) => {
                info!("BMI323 Chip ID: {:?}", buf);
                return Ok(buf);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    pub async fn read_chip_id(&mut self) -> Result<u16, I2C::Error> {
        let mut buf: [u8; 2] = [0; 2];
        let result = self.read_register_16(Register::CHIP_ID).await;
        match result {
            Ok(_) => {
                return Ok(u16::from_le_bytes(buf));
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    pub async fn read_error_status(&mut self) -> Result<BMI323Status, BMI323Status> {
        let mut buf = [0; 2];
        let result = self
            .i2c
            .write_read(self.address, &[Register::ERR_REG.addr()], &mut buf)
            .await;
        match result {
            Ok(_) => {
                if buf[0] == 0 {
                    info!("BMI323 Power Ok: {:?}", buf[0]);
                    return Ok(BMI323Status::POWER_OK);
                } else {
                    info!("BMI323 Power Error: {:?}", buf[0]);
                    return Err(BMI323Status::POWER_ERROR);
                }
            }
            Err(_e) => {
                info!("BMI323 I2C Error");
                return Err(BMI323Status::I2C_Error);
            }
        }
    }
    pub async fn read_sensor_status(&mut self) -> Result<BMI323Status, BMI323Status> {
        let mut buf = [0; 2];
        let result = self
            .i2c
            .write_read(self.address, &[Register::STATUS.addr()], &mut buf)
            .await;
        match result {
            Ok(_) => {
                if buf[0] == 1 {
                    info!("BMI323 Initialized: {:?}", buf[0]);
                    return Ok(BMI323Status::INIT_OK);
                } else {
                    info!("BMI323 Initialization Error: {:?}", buf[0]);
                    return Err(BMI323Status::INIT_ERROR);
                }
            }
            Err(e) => return Err(BMI323Status::I2C_Error),
        }
    }
    pub async fn check_init_status(&mut self) -> Result<BMI323Status, BMI323Status> {
        let chip_id = self.read_chip_id().await;

        match chip_id {
            Ok(result) => {
                info!("BMI323 Chip ID: {:?}", result);
            }
            Err(e) => {
                return Err(BMI323Status::I2C_Error);
            }
        }
        let device_status = self.read_error_status().await;
        let sensor_status = self.read_sensor_status().await;
        match device_status {
            Ok(_) => match sensor_status {
                Ok(_) => {
                    return Ok(BMI323Status::INIT_OK);
                }
                Err(_) => {
                    return Err(BMI323Status::INIT_ERROR);
                }
            },
            Err(_) => {
                return Err(BMI323Status::I2C_Error);
            }
        }
    }

    async fn read_acc_data_x(&mut self) -> Result<i16, I2C::Error> {
        let mut buf = [0; 2];
        let result = self
            .i2c
            .write_read(self.address, &[Register::ACC_DATA_X.addr()], &mut buf)
            .await;
        match result {
            Ok(_) => {
                return Ok(i16::from_le_bytes(buf));
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    async fn read_acc_data_y(&mut self) -> Result<i16, I2C::Error> {
        let mut buf = [0; 2];
        let result = self
            .i2c
            .write_read(self.address, &[Register::ACC_DATA_Y.addr()], &mut buf)
            .await;
        match result {
            Ok(_) => {
                return Ok(i16::from_le_bytes(buf));
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    async fn read_acc_data_z(&mut self) -> Result<i16, I2C::Error> {
        let mut buf = [0; 2];
        let result = self
            .i2c
            .write_read(self.address, &[Register::ACC_DATA_Z.addr()], &mut buf)
            .await;
        match result {
            Ok(_) => {
                return Ok(i16::from_le_bytes(buf));
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    async fn read_gyr_data_x(&mut self) -> Result<i16, I2C::Error> {
        let mut buf = [0; 2];
        let result = self
            .i2c
            .write_read(self.address, &[Register::GYR_DATA_X.addr()], &mut buf)
            .await;
        match result {
            Ok(_) => {
                return Ok(i16::from_le_bytes(buf));
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    async fn read_gyr_data_y(&mut self) -> Result<i16, I2C::Error> {
        let mut buf = [0; 2];
        let result = self
            .i2c
            .write_read(self.address, &[Register::GYR_DATA_Y.addr()], &mut buf)
            .await;
        match result {
            Ok(_) => {
                return Ok(i16::from_le_bytes(buf));
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    async fn read_gyr_data_z(&mut self) -> Result<i16, I2C::Error> {
        let mut buf = [0; 2];
        let result = self
            .i2c
            .write_read(self.address, &[Register::GYR_DATA_Z.addr()], &mut buf)
            .await;
        match result {
            Ok(_) => {
                return Ok(i16::from_le_bytes(buf));
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    async fn read_temp_data(&mut self) -> Result<i16, I2C::Error> {
        let mut buf = [0; 2];
        let result = self
            .i2c
            .write_read(self.address, &[Register::TEMP_DATA.addr()], &mut buf)
            .await;
        match result {
            Ok(_) => {
                return Ok(i16::from_le_bytes(buf));
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    async fn read_sensor_time_0(&mut self) -> Result<u16, I2C::Error> {
        let mut buf = [0; 2];
        let result = self
            .i2c
            .write_read(self.address, &[Register::SENSOR_TIME_O.addr()], &mut buf)
            .await;
        match result {
            Ok(_) => {
                return Ok(u16::from_le_bytes(buf));
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    async fn read_sensor_time_1(&mut self) -> Result<u16, I2C::Error> {
        let mut buf = [0; 2];
        let result = self
            .i2c
            .write_read(self.address, &[Register::SENSOR_TIME_1.addr()], &mut buf)
            .await;
        match result {
            Ok(_) => {
                return Ok(u16::from_le_bytes(buf));
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    async fn read_saturation_flags(&mut self) -> Result<u16, I2C::Error> {
        let mut buf = [0; 2];
        let result = self
            .i2c
            .write_read(self.address, &[Register::SAT_FLAGS.addr()], &mut buf)
            .await;
        match result {
            Ok(_) => {
                return Ok(u16::from_le_bytes(buf));
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Format)]
pub enum Register {
    CHIP_ID = 0x00,
    ERR_REG = 0x01,
    STATUS = 0x02,
    ACC_DATA_X = 0x03,
    ACC_DATA_Y = 0x04,
    ACC_DATA_Z = 0x05,
    GYR_DATA_X = 0x06,
    GYR_DATA_Y = 0x07,
    GYR_DATA_Z = 0x08,
    TEMP_DATA = 0x09,
    SENSOR_TIME_O = 0x0A,
    SENSOR_TIME_1 = 0x0B,
    SAT_FLAGS = 0x0C,
    INT_STATUS_INT1 = 0x0D,
    INT_STATUS_INT2 = 0x0E,
    INT_STATUS_IBI = 0x0F,
    FEATURE_I00 = 0x10,
    FEATURE_I01 = 0x11,
    FEATURE_I02 = 0x12,
    FEATURE_I03 = 0x13,
    FEATURE_IO_STATUS = 0x14,
    FIFO_FILL_LEVEL = 0x15,
    FIFO_DATA = 0x16,
    ACC_CONF = 0x20,
    GYR_CONF = 0x21,
    ALT_ACC_CONF = 0x28,
    ALT_GYR_CONF = 0x29,
    ALT_CONF = 0x2A,
    ALT_STATUS = 0x2B,
    FIFO_WATERMARK = 0x35,
    FIFO_CONF = 0x36,
    FIFO_CTRL = 0x37,
    IO_INT_CTRL = 0x38,
    INT_CONF = 0x39,
    INT_MAP1 = 0x3A,
    INT_MAP2 = 0x3B,
    FEATURE_CTRL = 0x40,
    FEATURE_DATA_ADDR = 0x41,
    FEATURE_DATA_TX = 0x42,
    FEATURE_DATA_STATUS = 0x43,
    FEATURE_ENGINE_STATUS = 0x45,
    FEATURE_EVENT_EXT = 0x47,
    IO_PDN_CTRL = 0x4F,
    IO_SPI_IF = 0x50,
    IO_PAD_STRENGTH = 0x51,
    IO_I2C_IF = 0x52,
    IO_ODR_DEVIATION = 0x53,
    ACC_DP_OFF_X = 0x60,
    ACC_DP_DGAIN_X = 0x61,
    ACC_DP_OFF_Y = 0x62,
    ACC_DP_DGAIN_Y = 0x63,
    ACC_DP_OFF_Z = 0x64,
    ACC_DP_DGAIN_Z = 0x65,
    GYR_DP_OFF_X = 0x66,
    GYR_DP_DGAIN_X = 0x67,
    GYR_DP_OFF_Y = 0x68,
    GYR_DP_DGAIN_Y = 0x69,
    GYR_DP_OFF_Z = 0x6A,
    GYR_DP_DGAIN_Z = 0x6B,
    I3C_TC_SYNC_TPH = 0x70,
    I3C_TC_SYNC_TU = 0x71,
    I3C_TC_SYNC_ODR = 0x72,
    CMD = 0x7E,
    CFG_RES = 0x7F,
}
impl Register {
    #[inline(always)]
    pub fn addr(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Format)]
pub enum BMI323Status {
    I2C_Error,
    POWER_ERROR,
    POWER_OK,
    INIT_ERROR,
    INIT_OK,
}
