use std::fs::{File, read_dir};
use std::io;
use std::io::Read;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::str::Utf8Error;

use log::info;

pub mod lsm6dsl;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    Io(io::Error),
    SensorNotFound(String),
    Parse(ParseIntError),
    Utf(Utf8Error),
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<Utf8Error> for Error {
    fn from(value: Utf8Error) -> Self {
        Self::Utf(value)
    }
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Self::Parse(value)
    }
}

/// Return the iio device directory for a given name, if it exists
pub(super) fn iio_device_directory(sensor_name: &str) -> Result<PathBuf, Error> {
    let found = read_dir("/sys/bus/iio/devices/")?
        .filter_map(|x| x.ok())
        .map(|x| println!("{}", String::from(x.path().to_owned().to_string_lossy())));

    todo!()
}
