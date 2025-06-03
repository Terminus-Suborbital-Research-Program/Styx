#![no_std]
// Library
#[cfg(feature = "sync")]
pub mod sync_mod;
#[cfg(feature = "sync")]
pub use sync_mod::*;
#[cfg(feature = "async")]
pub mod async_mod;
#[cfg(feature = "async")]
pub use async_mod::*;
mod calibration;

// Types/Constants
pub type Address = u8;
pub type Reset = u8;
pub type Register = (Address, Reset);
pub const DEFAULT_ADDRESS: u8 = 0x76;
pub const CHIP_ID: u8 = 0x60;
pub(crate) const REGISTER_CHIP_ID: u8 = 0xd0;
pub(crate) const SOFT_RESET: u8 = 0xe0;
pub(crate) const CONTROL_HUMID: u8 = 0xf2;
pub(crate) const STATUS: u8 = 0xf3;
pub(crate) const CONTROL: u8 = 0xf4;
pub(crate) const CONFIG: u8 = 0xf5;
pub(crate) const PRESSURE: u8 = 0xf7;
pub(crate) const TEMP: u8 = 0xfa;
pub(crate) const HUMID: u8 = 0xfd;
pub(crate) const CMD_SOFT_RESET: u8 = 0xb6;
pub(crate) const MODE_SLEEP: u8 = 0b00;
pub(crate) const TEMPERATURE_OUTPUT: u32 = 0x80000;
pub(crate) const PRESSURE_OUTPUT: u32 = 0x80000;
pub(crate) const HUMIDITY_OUTPUT: u16 = 0x8000;

/// Chip configuration
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Configuration {
    /// Standby time settings
    standby_time: StandbyTime,

    /// Filter settings
    filter: Filter,

    /// SPI3w option
    spi3w: bool,

    /// Temperature oversampling settings
    temperature_oversampling: Oversampling,

    /// Pressure oversampling settings
    pressure_oversampling: Oversampling,

    /// Humidity oversampling settings
    humidity_oversampling: Oversampling,

    /// Sensor mode
    sensor_mode: SensorMode,
}

impl From<&Configuration> for (Config, ControlMeasurement, ControlHumidity) {
    fn from(configuration: &Configuration) -> Self {
        let config = (
            configuration.standby_time,
            configuration.filter,
            configuration.spi3w,
        )
            .into();
        let control_measurement = (
            configuration.temperature_oversampling,
            configuration.pressure_oversampling,
            configuration.sensor_mode,
        )
            .into();
        let control_humidity = configuration.humidity_oversampling.into();
        (config, control_measurement, control_humidity)
    }
}

impl Configuration {
    /// Convert to low-level configuration items
    #[doc(hidden)]
    #[must_use]
    pub(crate) fn to_lowlevel_configuration(
        &self,
    ) -> (Config, ControlMeasurement, ControlHumidity) {
        self.into()
    }

    /// Set the standby time
    #[must_use]
    pub fn with_standby_time(mut self, standby_time: StandbyTime) -> Self {
        self.standby_time = standby_time;
        self
    }

    /// Set the filter
    #[must_use]
    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filter = filter;
        self
    }

    /// Set the SPI3w option
    #[doc(hidden)]
    #[allow(unused)]
    pub(crate) fn with_spi3w(mut self, spi3w: bool) -> Self {
        self.spi3w = spi3w;
        self
    }

    /// Set the oversampling factor for temperature
    #[must_use]
    pub fn with_temperature_oversampling(mut self, temperature_oversampling: Oversampling) -> Self {
        self.temperature_oversampling = temperature_oversampling;
        self
    }

    /// Set the oversampling factor for pressure
    #[must_use]
    pub fn with_pressure_oversampling(mut self, pressure_oversampling: Oversampling) -> Self {
        self.pressure_oversampling = pressure_oversampling;
        self
    }

    /// Set the oversampling factor for humidity
    #[must_use]
    pub fn with_humidity_oversampling(mut self, humidity_oversampling: Oversampling) -> Self {
        self.humidity_oversampling = humidity_oversampling;
        self
    }

    /// Set the sensor mode
    #[must_use]
    pub fn with_sensor_mode(mut self, sensor_mode: SensorMode) -> Self {
        self.sensor_mode = sensor_mode;
        self
    }

    /// Check if chip is in forced mode
    #[doc(hidden)]
    pub(crate) fn is_forced(&self) -> bool {
        self.sensor_mode == SensorMode::Forced
    }
}


/// Chip status
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Status {
    /// True if the sensor is performing a measurement
    measuring: bool,

    /// True if the sensor is performing calibration
    calibrating: bool,
}

impl Status {
    /// Return `true` if the chip is measuring data
    #[must_use]
    pub fn is_measuring(&self) -> bool {
        self.measuring
    }

    /// Return `true` if the chip is computing calibration data
    #[must_use]
    pub fn is_calibrating(&self) -> bool {
        self.calibrating
    }
}
impl From<u8> for Status {
    fn from(arg: u8) -> Self {
        Self {
            measuring: (arg & 0b0000_0100) != 0,
            calibrating: (arg & 0b0000_0001) != 0,
        }
    }
}
