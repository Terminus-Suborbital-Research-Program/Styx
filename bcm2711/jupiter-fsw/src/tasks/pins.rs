use std::{
    sync::{Arc, Mutex, Once},
    time::Duration,
};

use i2cdev::{
    core::I2CDevice as _,
    linux::{LinuxI2CDevice, LinuxI2CError},
};
use log::{debug, info, warn};

use common::indicators::{IndicatorStates, MalformedIndicatorError};

#[derive(Clone)]
pub struct IndicatorsReader {
    pin_states: Arc<Mutex<IndicatorStates>>,
}

static ATMEGA_ONCE: Once = Once::new();
impl IndicatorsReader {
    pub fn new<T: Into<Atmega>>(atmega: T) -> Self {
        let pins = Arc::new(Mutex::new(IndicatorStates::none()));
        let c = pins.clone();
        let atmega = atmega.into();
        ATMEGA_ONCE.call_once(|| {
            std::thread::spawn(move || {
                pin_states_thread(atmega, c);
            });
        });
        Self { pin_states: pins }
    }

    pub fn read(&self) -> IndicatorStates {
        *self.pin_states.lock().unwrap()
    }
}

/// ATMega abstraction
pub struct Atmega {
    device: LinuxI2CDevice,
}

impl From<LinuxI2CDevice> for Atmega {
    fn from(device: LinuxI2CDevice) -> Self {
        Self { device }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum IndicatorError {
    I2CError(LinuxI2CError),
    Malformed(MalformedIndicatorError),
}

impl From<LinuxI2CError> for IndicatorError {
    fn from(err: LinuxI2CError) -> Self {
        IndicatorError::I2CError(err)
    }
}
impl From<MalformedIndicatorError> for IndicatorError {
    fn from(err: MalformedIndicatorError) -> Self {
        IndicatorError::Malformed(err)
    }
}

#[allow(dead_code)]
impl Atmega {
    pub fn new(device: LinuxI2CDevice) -> Self {
        device.into()
    }

    pub fn pins(&mut self) -> Result<IndicatorStates, IndicatorError> {
        Ok(IndicatorStates::try_from(self.device.smbus_read_byte()?)?)
    }
}

// update signature to use SharedPinStates and simplify body
fn pin_states_thread(mut atmega: Atmega, pins: Arc<Mutex<IndicatorStates>>) -> ! {
    loop {
        match atmega.pins() {
            Ok(new_pins) => {
                let mut pin_states = pins.lock().unwrap();
                info!("New pin states: {new_pins:?}");
                *pin_states = new_pins;
            }

            Err(e) => {
                warn!("Error reading pins: {e:?}");
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}
