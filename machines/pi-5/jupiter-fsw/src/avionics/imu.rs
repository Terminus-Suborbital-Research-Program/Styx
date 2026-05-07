use linux_embedded_hal::{I2cdev, I2CError, Delay};
use adxl345_driver2::{i2c::Device, AdxlError, Adxl345Reader};
use bmi323::{Bmi323, AccelConfig, GyroConfig, OutputDataRate, AccelerometerRange, GyroscopeRange, interface::I2cInterface,
     Error as BmiError};
use adxl345_driver2::i2c::Device as AdxlDevice;
use bin_packets::packets::ApplicationPacket;
use log::{error, info};

const SCALE_MULTIPLIER: f32 = 0.049;

#[derive(Debug)]
pub enum IMUError {
    BusFailed(I2CError),
    SensorFailed(AdxlError),
    BMIFail(BmiError<I2CError>),
}

impl From<I2CError> for IMUError {
    fn from(err: I2CError) -> Self {
        IMUError::BusFailed(err)
    }
}

impl From<AdxlError> for IMUError {
    fn from(err: AdxlError) -> Self {
        IMUError::SensorFailed(err)
    }
}

impl From<BmiError<I2CError>> for IMUError {
    fn from(err: BmiError<I2CError>) -> Self {
        IMUError::BMIFail(err)
    }
}

pub struct HighGAccel {
    sensor: Device<I2cdev>,
}

impl HighGAccel {
    pub fn new(device: Device<I2cdev>) -> Self  {
        Self { sensor: device }
    }

    pub fn read_data(&mut self) -> Result<[f32; 3], IMUError> {
        let (raw_x, raw_y, raw_z) = self.sensor.acceleration()?;
        Ok([
            raw_x as f32 * SCALE_MULTIPLIER,
            raw_y as f32 * SCALE_MULTIPLIER,
            raw_z as f32 * SCALE_MULTIPLIER,
        ])
    }
}

pub struct AvionicsImuManager {
    bmi: Bmi323<I2cInterface<I2cdev>, Delay>,
    // high_g: HighGAccel,
}


#[derive(Default)]
pub struct IMU_Results {
    // pub high_range: Option<ApplicationPacket>,
    pub low_range: Option<ApplicationPacket>,
    pub gyro: Option<ApplicationPacket>,
}

impl AvionicsImuManager {
    pub fn new() -> Result<Self, IMUError> {
        let i2c_bmi = I2cdev::new("/dev/i2c-1").map_err(I2CError::from)?;
        // let i2c_adxl = I2cdev::new("/dev/i2c-1").map_err(I2CError::from)?;

        // let adxl_device = AdxlDevice::new(i2c_adxl)?;
        // let high_g = HighGAccel::new(adxl_device);

        let delay = Delay; 
        let mut bmi = Bmi323::new_with_i2c(i2c_bmi, 0x68, delay);
        bmi.init()?; 

        let accel_config = AccelConfig::builder()
            .odr(OutputDataRate::Odr100hz)
            .range(AccelerometerRange::G8)
            .build();
        bmi.set_accel_config(accel_config)?;

        let gyro_config = GyroConfig::builder()
            .odr(OutputDataRate::Odr100hz)
            .range(GyroscopeRange::DPS2000)
            .build();
        bmi.set_gyro_config(gyro_config)?;

        Ok(Self { bmi
            // , high_g 
        })
    }

    pub fn read_all(&mut self, startup: std::time::Instant) -> IMU_Results {
        
        let mut results = IMU_Results::default();

        let timestamp_ms: u64 = std::time::Instant::now()
                            .duration_since(startup)
                            .as_millis() as u64;
        
        // if let Ok(adxl_data) = self.high_g.read_data() {
        //     info!("High-G (ADXL375): x={}, y={}, z={}", adxl_data[0], adxl_data[1], adxl_data[2]);
            
        //     results.high_range = Some(ApplicationPacket::JupiterAccelerometer {
        //         timestamp_ms,
        //         vector: adxl_data,
        //     });
        // } else {
        //     error!("Failed to read High-G ADXL375");
        // }

        if let Ok(bmi_accel) = self.bmi.read_accel_data_scaled() {
            info!("Low-G (BMI323): x={}, y={}, z={}", bmi_accel.x, bmi_accel.y, bmi_accel.z);
            
            results.low_range = Some(ApplicationPacket::AccelerometerData { 
                timestamp: timestamp_ms, 
                x: bmi_accel.x, 
                y: bmi_accel.y, 
                z: bmi_accel.z
            });
        } else {
            error!("Failed to read Low-G BMI323 Accelerometer");
        }

        if let Ok(bmi_gyro) = self.bmi.read_gyro_data_scaled() {
            info!("Gyro (BMI323): x={}, y={}, z={}", bmi_gyro.x, bmi_gyro.y, bmi_gyro.z);
            
            results.gyro = Some(ApplicationPacket::GyroscopeData { 
                timestamp: timestamp_ms, 
                x: bmi_gyro.x, 
                y: bmi_gyro.y, 
                z: bmi_gyro.z
            });
        } else {
            error!("Failed to read BMI323 Gyroscope");
        }

        results
    }
}