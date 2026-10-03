//! Shared protocol types for the satellite firmware and host-side compute tool.

#![no_std]
#![warn(missing_docs)]

use heapless::String;

pub use bin_packets::phases::EjectedSatellitePhase;

/// Baud rate for the offboard compute UART link.
pub const COMPUTE_UART_BAUD: u32 = 115_200;

/// Baud rate for the GPS receiver UART link.
pub const GPS_UART_BAUD: u32 = 9_600;

/// Telemetry flag indicating the magnetometer produced a valid sample.
pub const MAG_VALID_FLAG: u8 = 0x01;

/// Telemetry flag indicating the IMU produced a valid sample.
pub const IMU_VALID_FLAG: u8 = 0x02;

/// Telemetry flag indicating a valid GPS fix is currently available.
pub const GPS_VALID_FLAG: u8 = 0x04;

/// Telemetry flag indicating the compute link has received at least one command.
pub const COMPUTE_LINK_FLAG: u8 = 0x08;

/// Latest parsed GPS fix state.
#[derive(Clone, Copy, Debug, Default)]
pub struct GpsFix {
    /// Total NMEA sentences seen since boot.
    pub sentence_count: u32,
    /// GPS fix quality from GGA, where `0` means invalid.
    pub fix_quality: u8,
    /// Satellite count reported by the receiver.
    pub satellites: u8,
    /// Latitude in signed decimal degrees.
    pub latitude_deg: f32,
    /// Longitude in signed decimal degrees.
    pub longitude_deg: f32,
    /// Altitude in meters above mean sea level.
    pub altitude_m: f32,
    /// Whether the last parsed sentence indicated a valid location fix.
    pub valid: bool,
}

/// Consolidated board telemetry sample.
#[derive(Clone, Copy, Debug, Default)]
pub struct TelemetrySnapshot {
    /// Monotonic telemetry sequence number.
    pub seq: u32,
    /// Monotonic timestamp in milliseconds since boot.
    pub timestamp_ms: u32,
    /// Validity flags for the sample.
    pub flags: u8,
    /// ICM-20948 accelerometer X axis in g.
    pub accel_x_g: f32,
    /// ICM-20948 accelerometer Y axis in g.
    pub accel_y_g: f32,
    /// ICM-20948 accelerometer Z axis in g.
    pub accel_z_g: f32,
    /// ICM-20948 gyroscope X axis in degrees per second.
    pub gyro_x_dps: f32,
    /// ICM-20948 gyroscope Y axis in degrees per second.
    pub gyro_y_dps: f32,
    /// ICM-20948 gyroscope Z axis in degrees per second.
    pub gyro_z_dps: f32,
    /// Number of compute commands received since boot.
    pub compute_rx_count: u32,
    /// ESC X commanded pulse width in microseconds.
    pub motor_x: f32,
    /// ESC Y commanded pulse width in microseconds.
    pub motor_y: f32,
    /// ESC Z commanded pulse width in microseconds.
    pub motor_z: f32,
}

impl TelemetrySnapshot {
}

/// Telemetry record sent over the mailbox to output tasks.
#[derive(Clone, Copy, Debug, Default)]
pub struct GncTelemetry {
    /// Active scripted maneuver stage index.
    pub maneuver_stage: u8,
    /// Estimated ICRF-to-body roll in radians.
    pub curr_roll: f32,
    /// Estimated ICRF-to-body pitch in radians.
    pub curr_pitch: f32,
    /// Estimated ICRF-to-body yaw in radians.
    pub curr_yaw: f32,
    /// Estimated body rate X in radians per second.
    pub curr_rate_x: f32,
    /// Estimated body rate Y in radians per second.
    pub curr_rate_y: f32,
    /// Estimated body rate Z in radians per second.
    pub curr_rate_z: f32,
    /// Commanded ICRF-to-body roll in radians.
    pub desired_roll: f32,
    /// Commanded ICRF-to-body pitch in radians.
    pub desired_pitch: f32,
    /// Commanded ICRF-to-body yaw in radians.
    pub desired_yaw: f32,
    /// Commanded body rate X in radians per second.
    pub desired_rate_x: f32,
    /// Commanded body rate Y in radians per second.
    pub desired_rate_y: f32,
    /// Commanded body rate Z in radians per second.
    pub desired_rate_z: f32,
    /// Commanded control torque X in newton-meters.
    pub torque_x: f32,
    /// Commanded control torque Y in newton-meters.
    pub torque_y: f32,
    /// Commanded control torque Z in newton-meters.
    pub torque_z: f32,
}

/// Complete telemetry frame carrying the latest snapshot, GPS fix, phase, and GNC output.
#[derive(Clone, Copy, Debug, Default)]
pub struct TelemetryFrame {
    /// Latest consolidated sensor snapshot.
    pub snapshot: TelemetrySnapshot,
    /// Active mission phase for the ejected satellite sequence.
    pub phase: EjectedSatellitePhase,
    /// Latest GNC-derived attitude, rate, and torque telemetry.
    pub gnc: GncTelemetry,
}

impl TelemetryFrame {
    /// Builds a telemetry frame from a snapshot and GPS fix.
    pub const fn new(
        snapshot: TelemetrySnapshot,
        phase: EjectedSatellitePhase,
    ) -> Self {
        Self {
            snapshot,
            phase,
            gnc: GncTelemetry {
                maneuver_stage: 0,
                curr_roll: 0.0,
                curr_pitch: 0.0,
                curr_yaw: 0.0,
                curr_rate_x: 0.0,
                curr_rate_y: 0.0,
                curr_rate_z: 0.0,
                desired_roll: 0.0,
                desired_pitch: 0.0,
                desired_yaw: 0.0,
                desired_rate_x: 0.0,
                desired_rate_y: 0.0,
                desired_rate_z: 0.0,
                torque_x: 0.0,
                torque_y: 0.0,
                torque_z: 0.0,
            },
        }
    }
}

/// Parsed command received from the offboard compute UART link.
#[derive(Clone, Debug)]
pub enum ComputeMessage {
    /// Ping request carrying the original sequence token.
    Ping(String<32>),
    /// Request to emit the latest telemetry frame immediately.
    Snapshot,
    /// Set the on-board LED state.
    SetLed(bool),
    /// Set all three ESC commands as absolute pulse widths in microseconds.
    SetMotors([u16; 3]),
    /// Echo the provided payload back to the sender.
    Echo(String<96>),
    /// Command line that did not match a known protocol verb.
    Unknown(String<96>),
}
