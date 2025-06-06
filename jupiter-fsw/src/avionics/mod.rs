use std::fs::{File, read_dir};
use std::io;
use std::io::Read;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::str::Utf8Error;

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
    let read_dirs: Result<Vec<_>, _> = read_dir("/sys/bus/iio/devices/")?.collect();
    let iio_paths_and_names: Result<Vec<_>, io::Error> = read_dirs?
        .iter()
        .filter_map(|x| {
            let mut name_path = x.path();
            name_path.push("name");
            match File::open(&name_path) {
                Ok(file) => Some((file, x.path())),
                Err(_) => None,
            }
        })
        .map(|mut file| {
            let mut buff = Vec::new();
            file.0.read_to_end(&mut buff)?;
            Ok((String::from_utf8_lossy(&buff).into_owned(), file.1))
        })
        .collect();

    if let Some(path) = iio_paths_and_names?.into_iter().find_map(|(name, path)| {
        if name == sensor_name {
            Some(path)
        } else {
            None
        }
    }) {
        Ok(path)
    } else {
        Err(Error::SensorNotFound(sensor_name.into()))
    }
}
