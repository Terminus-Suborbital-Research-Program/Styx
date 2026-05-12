//! Device constants and type definitions for the Ejector

#![warn(missing_docs, clippy::unwrap_used)]

use pins::{EjectionPin, JupiterRxPin, JupiterTxPin, OnboardLEDPin};
use rp235x_hal::{
    gpio::{FunctionI2C, FunctionSio, Pin, PullDown, PullNone, PullUp, SioInput, SioOutput},
    i2c::{Controller, Peripheral},
    pac::{I2C0, I2C1, UART0, UART1},
    timer::CopyableTimer1,
    uart::{Enabled, Reader, UartPeripheral, Writer},
    Timer, I2C,
};

#[allow(dead_code)]
pub mod pins {
    use rp235x_hal::gpio::{
        bank0::{
            *
        },
        FunctionI2C, FunctionSio, FunctionUart, Pin, PullDown, PullUp, SioInput, SioOutput,
    };

    /// Ejector Heartbeat Output
    pub type OnboardLEDPin = Gpio25;

    // Camera Startup should be right but the heartbeat and Cam LED Pins might be wrong
    // (inconsistency in ejector pinout doc) ask Brooks later

    pub type Cam1Pin = Gpio10;

    pub type Cam2Pin = Gpio11;

    /// Camera GPIO activation
    pub type CamMosfetPin = Pin<Gpio12, FunctionSio<SioOutput>, PullDown>;

    // pub type RGBLedPin = Gpio26;
    pub type RGBLedPin = Gpio24;

    /// RBF PIN
    pub type RBFPin = Pin<Gpio42, FunctionSio<SioInput>, PullDown>;

    /// Ejection detection pin
    pub type EjectionPin = Gpio38;

    /// UART RX
    pub type JupiterRxPin = Pin<Gpio1, FunctionUart, PullDown>;
    /// UART TX
    pub type JupiterTxPin = Pin<Gpio0, FunctionUart, PullDown>;

    /// I2C SDA pin
    pub type ThermoI2CSdaPin = Gpio32;
    /// I2C SCL pin
    pub type ThermoI2CSclPin = Gpio33;

    // /// GUARD SDA
    // pub type GuardSda = Pin<Gpio26, FunctionI2C, PullUp>;
    // /// GUARD SCL
    // pub type GuardScl = Pin<Gpio27, FunctionI2C, PullUp>;
}

pub use pins::*;
/// I2C bus for the thermocouple
pub type ThermoI2cBus = I2C<
    I2C0,
    (
        Pin<ThermoI2CSdaPin, FunctionI2C, PullUp>,
        Pin<ThermoI2CSclPin, FunctionI2C, PullUp>,
    ),
    Controller,
>;

// SI1145
//pub type GuardI2C = I2C<I2C1, (GuardSda, GuardScl), Controller>;

pub type SDCardPins = u8;

// Heartbeat LED
pub type OnboardLED = Pin<OnboardLEDPin, FunctionSio<SioOutput>, PullNone>;

/// Camera LED
// pub type RedLed = Pin<RedLedPin, FunctionSio<SioOutput>, PullNone>;
pub type Cam1 = Pin<Cam1Pin, FunctionSio<SioOutput>, PullNone>;
pub type Cam2 = Pin<Cam2Pin, FunctionSio<SioOutput>, PullNone>;

/// Camera LED
// pub type GreenLed = Pin<GreenLedPin, FunctionSio<SioOutput>, PullNone>;

pub type RGBLed = Pin<RGBLedPin, FunctionSio<SioOutput>, PullNone>;

/// Ejection detection pin
pub type EjectionDetectionPin = Pin<EjectionPin, FunctionSio<SioInput>, PullDown>;

/// JUPITER Uart
// pub type JupiterUart = UartPeripheral<Enabled, UART0, (JupiterRxPin, JupiterTxPin)>;

// pub type JupiterRX = Reader<UART0, (JupiterRxPin, JupiterTxPin)>;

// pub type JupiterTX = Writer<UART0, (JupiterRxPin, JupiterTxPin)>;

pub type JupiterUart = UartPeripheral<Enabled, UART0, (JupiterTxPin, JupiterRxPin)>;

// Update these as well to match the new JupiterUart tuple order
pub type JupiterRX = Reader<UART0, (JupiterTxPin, JupiterRxPin)>;
pub type JupiterTX = Writer<UART0, (JupiterTxPin, JupiterRxPin)>;

/// Samples per second of the geiger counter
pub static SAMPLE_COUNT: usize = 100;

use smart_leds::RGB8;

pub struct RGBStatus {
    pub RBF: RGB8,
    pub HaLow: RGB8,
    pub Esp: RGB8,
    pub Infratracker: RGB8,
    pub Guard: RGB8,
    pub Jupiter: RGB8,
    pub ElectroMagnet: RGB8,
    pub Servos: RGB8,
    pub Jupiter_Avionics_Health: RGB8,
    pub Ejector_Health: RGB8,
    pub Odin_Compute_Health: RGB8,
    pub Odin_Pico_Health: RGB8,
}

use bin_packets::rgbstatus::{RGBOptions, WireColor};

impl RGBStatus {
    // Convert recieved binpacket colors to actual color
    pub fn update_from_options(&mut self, options: RGBOptions) {
        if let Some(c) = options.RBF {
            self.RBF = c.into();
        }
        if let Some(c) = options.HaLow {
            self.HaLow = c.into();
        }
        if let Some(c) = options.Esp {
            self.Esp = c.into();
        }
        if let Some(c) = options.Infratracker {
            self.Infratracker = c.into();
        }
        if let Some(c) = options.Guard {
            self.Guard = c.into();
        }
        if let Some(c) = options.Jupiter {
            self.Jupiter = c.into();
        }
        if let Some(c) = options.ElectroMagnet {
            self.ElectroMagnet = c.into();
        }
        if let Some(c) = options.Servos {
            self.Servos = c.into();
        }
        if let Some(c) = options.Jupiter_Avionics_Health {
            self.Jupiter_Avionics_Health = c.into();
        }
        if let Some(c) = options.Ejector_Health {
            self.Ejector_Health = c.into();
        }
        if let Some(c) = options.Odin_Compute_Health {
            self.Odin_Compute_Health = c.into();
        }
        if let Some(c) = options.Odin_Pico_Health {
            self.Odin_Pico_Health = c.into();
        }
    }
}

impl Default for RGBStatus {
    fn default() -> Self {
        let dim_red     = RGB8::new(50, 0, 0);
        let dim_green   = RGB8::new(0, 50, 0);
        let dim_blue    = RGB8::new(0, 0, 50);

        let dim_yellow  = RGB8::new(40, 40, 0);
        let dim_cyan    = RGB8::new(0, 40, 40);
        let dim_magenta = RGB8::new(40, 0, 40);

        let dim_orange  = RGB8::new(50, 20, 0);
        let dim_purple  = RGB8::new(25, 0, 50);
        let dim_white   = RGB8::new(30, 30, 30);
        let off         = RGB8::new(0, 0, 0);
        
        Self {
            RBF: off,
            HaLow: off,
            Esp: off,
            Infratracker: off,
            Guard: off,
            Jupiter: off,
            ElectroMagnet: off,
            Servos: off,
            Jupiter_Avionics_Health: off,
            Ejector_Health: off,
            Odin_Compute_Health: off,
            Odin_Pico_Health: off,
        }
    }
}

pub const COLOR_DIM_RED: RGB8     = RGB8::new(50, 0, 0);
pub const COLOR_DIM_GREEN: RGB8   = RGB8::new(0, 50, 0);
pub const COLOR_DIM_BLUE: RGB8    = RGB8::new(0, 0, 50);
pub const COLOR_DIM_MAGENTA: RGB8 = RGB8::new(50, 0, 50);
pub const COLOR_OFF: RGB8         = RGB8::new(0, 0, 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagnetState {
    Off = 0,
    Holding = 1,
    Ejecting = 2,
    Unknown,
}

impl From<u8> for MagnetState {
    fn from(val: u8) -> Self {
        match val {
            0 => MagnetState::Off,
            1 => MagnetState::Holding,
            2 => MagnetState::Ejecting,
            _ => MagnetState::Unknown,
        }
    }
}

impl MagnetState {
    pub fn color(&self) -> RGB8 {
        match self {
            MagnetState::Off => COLOR_OFF,
            MagnetState::Holding => COLOR_DIM_BLUE,
            MagnetState::Ejecting => COLOR_DIM_MAGENTA,
            MagnetState::Unknown => COLOR_OFF,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServoState {
    Off = 0,
    PowerOn = 1,
    Release = 2,
    Unknown,
}

impl From<u8> for ServoState {
    fn from(val: u8) -> Self {
        match val {
            0 => ServoState::Off,
            1 => ServoState::PowerOn,
            2 => ServoState::Release,
            _ => ServoState::Unknown,
        }
    }
}

impl ServoState {
    pub fn color(&self) -> RGB8 {
        match self {
            ServoState::Off => COLOR_OFF,
            ServoState::PowerOn => COLOR_DIM_GREEN,
            ServoState::Release => COLOR_DIM_MAGENTA,
            ServoState::Unknown => COLOR_DIM_MAGENTA,
        }
    }
}



use mcp9600::{
    ADCResolution, BurstModeSamples, ColdJunctionResolution, DeviceAddr, FilterCoefficient,
    ShutdownMode, ThermocoupleType, MCP9600,
};
use bme280::i2c::BME280;
use bin_packets::packets::ApplicationPacket;
use defmt::{warn, error, info};
use heapless::Vec;


pub struct ThermocoupleChannel {
    pub id: u8,
    pub address: DeviceAddr,
}

pub struct SensorI2cManager {
    pub bus: ThermoI2cBus,
    pub tc_channels: [ThermocoupleChannel; 5],
    pub timer: Timer<CopyableTimer1>,
}

impl SensorI2cManager {
    pub fn new(mut bus: ThermoI2cBus, timer: Timer<CopyableTimer1>) -> Self {
        // Ids mapped from lowest id to lowest addr, and upwards
        let tc_channels = [
            ThermocoupleChannel { id: 1, address: DeviceAddr::AD3 },
            ThermocoupleChannel { id: 2, address: DeviceAddr::AD0 },
            ThermocoupleChannel { id: 3, address: DeviceAddr::AD1 },
            ThermocoupleChannel { id: 4, address: DeviceAddr::AD2 },
            ThermocoupleChannel { id: 5, address: DeviceAddr::AD7 },
        ];

        // Temporarily instantiate driver to configure
        for ch in &tc_channels {
            if let Ok(mut sensor) = MCP9600::new(&mut bus, ch.address) {
                info!("Successfully initialized MCP9600 CH{} at address {:?}", ch.id, ch.address as u8);

                let _ = sensor.set_sensor_configuration(ThermocoupleType::TypeK, FilterCoefficient::FilterMedium);
                let _ = sensor.set_device_configuration(
                    ColdJunctionResolution::High,
                    ADCResolution::Bit18,
                    BurstModeSamples::Sample1,
                    ShutdownMode::NormalMode,
                );
            } else {
                warn!("Failed to initialize MCP9600 CH{} at address {:?}", ch.id, ch.address as u8);
            }
        }

        Self { bus, tc_channels, timer }
    }

    /// Read all 5 thermocouples
    pub fn poll_thermocouples(&mut self, timestamp: u64) -> Vec<ApplicationPacket, 5> {
        let mut packets = Vec::new();
        
        // Re-instantiate because mcp9600 is literally just a ref to an i2c bus, and the address to send to. 
        // This way we don't have to deal with any locks or sharing abstractions around the bus 
        for ch in &self.tc_channels {
            if let Ok(mut sensor) = MCP9600::new(&mut self.bus, ch.address) {
                match sensor.read_hot_junction() {
                    Ok(temp) => {
                        let _ = packets.push(ApplicationPacket::ThermocoupleData {
                            timestamp,
                            channel: ch.id,
                            hot_junction_temp: temp,
                        });
                    }
                    Err(_) => warn!("Failed to read MCP9600 CH{}", ch.id),
                }
            }
        }
        packets
    }

    pub fn poll_bme280(&mut self, timestamp: u64) -> Option<ApplicationPacket> {
        let mut bme = BME280::new_primary(&mut self.bus);
        
        if bme.init(&mut self.timer).is_ok() {
            if let Ok(measurements) = bme.measure(&mut self.timer) {
                return Some(ApplicationPacket::BMEData {
                    timestamp,
                    temperature: measurements.temperature,
                    pressure: measurements.pressure,
                    humidity: measurements.humidity,
                });
            }
        }
        
        warn!("Failed to read BME280");
        None
    }
}