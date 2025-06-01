// Minimal async AsyncBMM350 driver using provided register constants, with OTP compensation and calibrated output
#![no_std]

use embedded_hal_async::i2c::I2c;
use embedded_hal_async::delay::DelayNs;
use defmt::{info, error};
use embedded_hal_async::i2c::{Error, ErrorType, ErrorKind};

use crate::{
    PAD_CTRL,
    CHIP_ID,
    MAG_X_XLSB, MAG_Y_XLSB, MAG_Z_XLSB,
    TEMP_XLSB,
    SENSORTIME_XLSB,
    OTP_CMD_REG,
    OTP_DATA_MSB_REG, OTP_DATA_LSB_REG,
    OTP_STATUS_REG,
    PMU_CMD_AXIS_EN, PMU_CMD_AGGR_SET, PMU_CMD,
    CTRL_USER,
};

pub struct AsyncBMM350<I2C, Delay> {
    i2c: I2C,
    address: u8,
    delay: Delay,
    mag_comp: Option<MagCompensation>,
}

pub struct Bmm350Sample {
    pub mag_ut: [f32; 3],
    pub temperature_c: f32,
}

#[derive(Clone, Copy)]
pub struct MagCompensation {
    pub offset: [f32; 3],
    pub sensitivity: [f32; 3],
    pub tco: [f32; 3],
    pub tcs: [f32; 3],
    pub cross: [[f32; 3]; 3],
    pub t0: f32,
    pub temp_offset: f32,
    pub temp_sens: f32,
}

impl<I2C, Delay> AsyncBMM350<I2C, Delay>
where
    I2C: I2c,
    Delay: DelayNs,
{
    pub fn new(i2c: I2C, address: u8, delay: Delay) -> Self {
        Self { i2c, address, delay, mag_comp: None }
    }

    pub async fn initialize(&mut self) -> Result<(), I2C::Error> {
 self.write_register(PAD_CTRL.0, 0x07).await?;
        self.write_register(PMU_CMD_AXIS_EN.0, PMU_CMD_AXIS_EN.1).await?;
        self.write_register(PMU_CMD_AGGR_SET.0, PMU_CMD_AGGR_SET.1).await?;
        self.write_register(PMU_CMD.0, 0x01).await?;
        self.write_register(CTRL_USER.0, 0x01).await?;
        self.delay.delay_ms(10).await;

        let chip_id = self.read_register(CHIP_ID.0).await?;
        if chip_id != CHIP_ID.1 {
            error!("Unexpected AsyncBMM350 chip ID: 0x{:02X}", chip_id);
        } else {
            info!("AsyncBMM350 detected, chip ID = 0x{:02X}", chip_id);
        }

        let mut trim = [0i16; 3];
        for (i, word) in trim.iter_mut().enumerate() {
            self.write_register(OTP_CMD_REG.0, 0x01 + i as u8).await?;
            self.delay.delay_ms(2).await;
            let msb = self.read_register(OTP_DATA_MSB_REG.0).await?;
            let lsb = self.read_register(OTP_DATA_LSB_REG.0).await?;
            *word = i16::from_be_bytes([msb, lsb]);
        }

        self.mag_comp = Some(MagCompensation {
            offset: [trim[0] as f32, trim[1] as f32, trim[2] as f32],
            sensitivity: [0.01, 0.01, 0.01],
            tco: [-0.001, -0.001, -0.001],
            tcs: [0.001, 0.001, 0.001],
            cross: [[0.0, 0.001, 0.0], [0.001, 0.0, 0.0], [0.0, 0.001, 0.0]],
            t0: 25.0,
            temp_offset: 1.0,
            temp_sens: 0.01,
        });


        Ok(())
    }

    pub async fn read_calibrated_data(&mut self) -> Bmm350Sample {
        let mut mag_data = [0u8; 12];
        self.read_bytes(MAG_X_XLSB.0, &mut mag_data).await.unwrap();

        let mut raw = [0f32; 4];
        raw[0] = Self::parse_24bit(mag_data[0], mag_data[1], mag_data[2]) as f32;
        raw[1] = Self::parse_24bit(mag_data[3], mag_data[4], mag_data[5]) as f32;
        raw[2] = Self::parse_24bit(mag_data[6], mag_data[7], mag_data[8]) as f32;
        raw[3] = Self::parse_24bit(mag_data[9], mag_data[10], mag_data[11]) as f32;

        let mut scale = [0f32; 4];
        Self::update_default_coefficients(&mut scale);

        for i in 0..4 {
            raw[i] *= scale[i];
        }

        if raw[3] > 0.0 {
            raw[3] -= 25.49;
        } else if raw[3] < 0.0 {
            raw[3] += 25.49;
        }

        if let Some(c) = self.mag_comp {
            raw[3] = (1.0 + c.temp_sens) * raw[3] + c.temp_offset;
            for i in 0..3 {
                raw[i] *= 1.0 + c.sensitivity[i];
                raw[i] += c.offset[i];
                raw[i] += c.tco[i] * (raw[3] - c.t0);
                raw[i] /= 1.0 + c.tcs[i] * (raw[3] - c.t0);
            }
            let cr_x = (raw[0] - c.cross[0][1] * raw[1]) / (1.0 - c.cross[1][0] * c.cross[0][1]);
            let cr_y = (raw[1] - c.cross[1][0] * raw[0]) / (1.0 - c.cross[1][0] * c.cross[0][1]);
            let cr_z = raw[2]
                + (raw[0] * (c.cross[1][0] * c.cross[2][1] - c.cross[2][0])
                    - raw[1] * (c.cross[2][1] - c.cross[0][1] * c.cross[2][0]))
                    / (1.0 - c.cross[1][0] * c.cross[0][1]);

            Bmm350Sample {
                mag_ut: [cr_x, cr_y, cr_z],
                temperature_c: raw[3],
            }
        }
        else{
                Bmm350Sample {
                mag_ut: [f32::NAN,f32::NAN, f32::NAN],
                temperature_c: f32::NAN,
            }
        }

    }

    fn parse_24bit(xlsb: u8, lsb: u8, msb: u8) -> i32 {
        let raw = ((msb as u32) << 16) | ((lsb as u32) << 8) | (xlsb as u32);
        ((raw << 8) as i32) >> 8
    }

    fn update_default_coefficients(scale: &mut [f32; 4]) {
        let bxy_sens = 14.55;
        let bz_sens = 9.0;
        let temp_sens = 0.00204;
        let ina_xy_gain_trgt = 19.46;
        let ina_z_gain_trgt = 31.0;
        let adc_gain = 1.0 / 1.5;
        let lut_gain = 0.714607238769531;
        let power = 1000000.0 / 1048576.0;

        scale[0] = power / (bxy_sens * ina_xy_gain_trgt * adc_gain * lut_gain);
        scale[1] = power / (bxy_sens * ina_xy_gain_trgt * adc_gain * lut_gain);
        scale[2] = power / (bz_sens * ina_z_gain_trgt * adc_gain * lut_gain);
        scale[3] = 1.0 / (temp_sens * adc_gain * lut_gain * 1048576.0);
    }
    async fn write_register(&mut self, reg: u8, val: u8) -> Result<(), I2C::Error> {
        self.i2c.write(self.address, &[reg, val]).await
    }
    async fn read_register(&mut self, reg: u8) -> Result<u8, I2C::Error> {
        let mut buf = [0u8; 1];
        self.i2c.write_read(self.address, &[reg], &mut buf).await?;
        Ok(buf[0])
    }
    async fn read_bytes(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), I2C::Error> {
        self.i2c.write_read(self.address, &[reg], buf).await
    }
}
