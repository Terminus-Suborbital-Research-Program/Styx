// Minimal async AsyncBMI323 FIFO-only driver (clean start)
#![no_std]

use embedded_hal_async::i2c::I2c;
use embedded_hal_async::delay::DelayNs;
use defmt::{info, error};

pub struct SampledMotion {
    pub timestamp_ticks: u32,
    pub accel: Option<[f32; 3]>, // m/s^2
    pub gyro: Option<[f32; 3]>,  // rad/s
}

pub struct FifoSample {
    pub kind: FifoFrameKind,
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

pub enum FifoFrameKind {
    Accel,
    Gyro,
    Both,
    Unknown(u8),
}

pub struct AsyncBMI323<I2C, Delay> {
    i2c: I2C,
    address: u8,
    delay: Delay,
}



impl<I2C, Delay> AsyncBMI323<I2C, Delay>
where
    I2C: I2c,
    Delay: DelayNs,
{
    pub fn new(i2c: I2C, address: u8, delay: Delay) -> Self {
        Self { i2c, address, delay }
    }

    pub async fn initialize(&mut self) -> Result<(), I2C::Error> {
        let chip_id = self.read_chip_id().await?;
        if chip_id != 0x43 {
            error!("Unexpected chip ID: 0x{:02X}", chip_id);
            // return Err(I2C::Error::Other);
        } else {
            info!("AsyncBMI323 detected, chip ID = 0x{:02X}", chip_id);
        }

        self.write_register(0x59, 0x0000).await?; // Config mode
        self.delay.delay_ms(2).await;

        self.write_register(0x20, 0x0028).await?; // ACC_CONF
        self.write_register(0x21, 0x0048).await?; // GYR_CONF

        // Correct CMD register writes using little endian
        self.i2c.write(self.address, &[0x7E, 0x00, 0x03]).await?; // CMD_ACC_ENABLE
        self.delay.delay_ms(5).await;
        self.i2c.write(self.address, &[0x7E, 0x00, 0x04]).await?; // CMD_GYR_ENABLE
        self.delay.delay_ms(5).await;

        self.write_register(0x36, 0x0003).await?; // FIFO_CONF: header + data
        self.write_register(0x37, 0x0002).await?; // FIFO_CTRL: stream mode
        self.delay.delay_ms(5).await;

        self.i2c.write(self.address, &[0x7E, 0x00, 0x01]).await?; // CMD_NORMAL_MODE
        self.delay.delay_ms(10).await;

        Ok(())
    }

    pub async fn read_fifo_frame(&mut self) -> Result<[u8; 32], I2C::Error> {
        let mut buf = [0u8; 32];
        self.read_bytes(0x16, &mut buf).await?; // FIFO_DATA
        Ok(buf)
    }

    pub fn parse_fifo_frame(buffer: &[u8]) -> Option<FifoSample> {
        if buffer.len() < 7 {
            return None;
        }

        let header = buffer[0];
        let x = i16::from_le_bytes([buffer[1], buffer[2]]);
        let y = i16::from_le_bytes([buffer[3], buffer[4]]);
        let z = i16::from_le_bytes([buffer[5], buffer[6]]);

        let kind = match header {
            0x84 => FifoFrameKind::Accel,
            0x88 => FifoFrameKind::Gyro,
            0x8C => FifoFrameKind::Both,
            _ => FifoFrameKind::Unknown(header),
        };

        Some(FifoSample { kind, x, y, z })
    }

    async fn write_register(&mut self, reg: u8, val: u16) -> Result<(), I2C::Error> {
        let bytes = [(val >> 8) as u8, (val & 0xFF) as u8];
        self.i2c.write(self.address, &[reg, bytes[0], bytes[1]]).await
    }

    async fn read_bytes(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), I2C::Error> {
        self.i2c.write_read(self.address, &[reg], buf).await
    }

    pub async fn read_chip_id(&mut self) -> Result<u8, I2C::Error> {
        let mut buf = [0u8; 1];
        self.read_bytes(0x00, &mut buf).await?;
        Ok(buf[0])
    }


    pub async fn sample(&mut self) -> Result<Option<SampledMotion>, I2C::Error> {
        let buf = self.read_fifo_frame().await?;

        let mut i = 0;
        while i + 7 <= buf.len() {
            let frame = &buf[i..i + 7];
            if let Some(sample) = Self::parse_fifo_frame(frame) {
                let mut time_buf = [0u8; 3];
                for (i, b) in time_buf.iter_mut().enumerate() {
                    self.read_bytes(0x0E + i as u8, core::slice::from_mut(b)).await?;
                }
                let sensor_time: u32 = ((time_buf[2] as u32) << 16)
                    | ((time_buf[1] as u32) << 8)
                    | (time_buf[0] as u32);

                let g_to_ms2 = 9.80665 / 16384.0; // scale for ±2g, 16-bit
                let dps_to_rads = core::f32::consts::PI / (180.0 * 131.0); // example scale for ±250 dps

                let accel = match sample.kind {
                    FifoFrameKind::Accel | FifoFrameKind::Both => Some([
                        sample.x as f32 * g_to_ms2,
                        sample.y as f32 * g_to_ms2,
                        sample.z as f32 * g_to_ms2,
                    ]),
                    _ => None,
                };

                let gyro = match sample.kind {
                    FifoFrameKind::Gyro | FifoFrameKind::Both => Some([
                        sample.x as f32 * dps_to_rads,
                        sample.y as f32 * dps_to_rads,
                        sample.z as f32 * dps_to_rads,
                    ]),
                    _ => None,
                };

                return Ok(Some(SampledMotion {
                    timestamp_ticks: sensor_time,
                    accel,
                    gyro,
                }));
            }
            i += 1;
        }
        Ok(None)
    }
}
