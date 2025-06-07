use std::fs::{File, read_dir};
use std::io;
use std::io::Read;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::str::Utf8Error;
use std::string::FromUtf8Error;

pub mod lsm6dsl;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    Io(io::Error),
    SensorNotFound(String),
    Parse(ParseIntError),
    Utf(FromUtf8Error),
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(value: std::string::FromUtf8Error) -> Self {
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
    match read_dir("/sys/bus/iio/devices/")?
        .filter_map(|x| x.ok())
        .map(|x| x.path())
        .map(|mut x| {
            x.push("name");
            x
        })
        .filter_map(|name_path| {
            if let Ok(file) = File::open(&name_path) {
                Some((file, name_path))
            } else {
                None
            }
        })
        .find_map(|mut pair| {
            let mut buffer = Vec::new();
            pair.0.read_to_end(&mut buffer).ok();
            if String::from_utf8_lossy(&buffer).trim() == sensor_name {
                Some(pair.1.parent().unwrap().to_owned())
            } else {
                None
            }
        }) {
        Some(val) => Ok(val),
        _ => Err(Error::SensorNotFound(sensor_name.into())),
    }
}
