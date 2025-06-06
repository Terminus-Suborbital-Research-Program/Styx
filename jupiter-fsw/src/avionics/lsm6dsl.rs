use std::{fs::File, io::Read, path::PathBuf};

use super::iio_device_directory;

const ACCEL_NAME: &str = "lsm6dsl_accel";

/// IIO Device LSM6DSL accelerometer
pub struct Lsm6DslAccel {
    iio_device_path: PathBuf,
}

#[allow(dead_code)]
impl Lsm6DslAccel {
    pub fn new() -> Result<Self, super::Error> {
        Ok(Self {
            iio_device_path: iio_device_directory(ACCEL_NAME)?,
        })
    }

    pub fn read_data(&self) -> Result<Vec<f32>, super::Error> {
        let mut x_path: PathBuf = self.iio_device_path.clone();
        x_path.push("in_accel_x_raw");
        let mut y_path: PathBuf = self.iio_device_path.clone();
        y_path.push("in_accel_y_raw");
        let mut z_path: PathBuf = self.iio_device_path.clone();
        z_path.push("in_accel_x_raw");

        let paths = [x_path, y_path, z_path];
        let mut readings = Vec::new();
        for path in paths {
            let mut buffer = Vec::new();
            File::open(path)?.read_to_end(&mut buffer)?;

            let value = str::from_utf8(&buffer)?.parse::<i32>()?;
            println!("{value}");
            readings.push(value as f32 * 0.061 / 1000.0);
        }

        Ok(readings)
    }
}
