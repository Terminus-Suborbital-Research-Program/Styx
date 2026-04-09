//! Wi-Fi enabled INS bring-up with UDP state streaming.

use core::{cell::RefCell, fmt::Write as _};

use aether_core::matrix;
use aether_proprietary::navigation::{AbsoluteNavigator, AttitudeState, GeodeticHotStart};
use aether_models::{
    attitude::{DirectionCosineMatrix, Euler, Quaternion},
    coordinate::Cartesian,
    real::Real,
    reference_frame::{
        transforms::{body_to_ned, itrf_to_icrf, itrf_to_ned},
        Body, ICRF, ITRF, NED, RotatingFrame,
    },
    terrestrial::wgs84::transforms::geocentric_to_ecef,
};
use cyw43::{JoinOptions, aligned_bytes};
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use defmt::{info, warn};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{Config as NetConfig, DhcpConfig, Ipv4Address, StackResources};
use embassy_rp::gpio::{Input, Level, Output, OutputOpenDrain, Pull};
use embassy_rp::i2c::{self, I2c};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::uart::{self, Blocking, Uart};
use embassy_rp::{bind_interrupts, dma};
use embassy_time::{Delay, Duration, Timer, block_for};
use embedded_hal_0_2::{
    blocking::i2c::{Read as I2cRead, Write as I2cWrite, WriteRead as I2cWriteRead},
};
use heapless::String;
use icm20948::{ICMError, ICMI2C, ICM20948_CHIP_ADR, ICM20948_CHIP_ADR_ALT};
use ms8607::MS8607;
use nmea::Nmea;
use panic_halt as _;
use static_cell::StaticCell;

const GPS_BAUD_HZ: u32 = 9_600;
const SENSOR_POLL_DIVIDER: u32 = 5;
const NMEA_SENTENCE_CAPACITY: usize = 128;
const LAST_NMEA_SENTENCE_CAPACITY: usize = 96;
const STATE_MESSAGE_CAPACITY: usize = 1024;
const ICM_WHO_AM_I_REG: u8 = 0x00;
const ICM_PWR_MGMT_1_REG: u8 = 0x06;
const ICM_PWR_MGMT_2_REG: u8 = 0x07;
const ICM_INT_PIN_CFG_REG: u8 = 0x0F;
const ICM_INT_PIN_CFG_BYPASS_ENABLE: u8 = 0x30;
const AK09916_I2C_ADDR: u8 = 0x0C;
const AK09916_ST1_REG: u8 = 0x10;
const AK09916_HXL_REG: u8 = 0x11;
const AK09916_CNTL2_REG: u8 = 0x31;
const AK09916_MODE_CONTINUOUS_100HZ: u8 = 0x08;
const AK09916_UT_PER_LSB: f32 = 0.15;
const WIFI_NETWORK: &str = "Student5";
const WIFI_PASSWORD: &str = "Go Chargers!";
const WIFI_HOSTNAME: &str = "ins-test";
const UDP_STREAM_PORT: u16 = 4242;
const UDP_SOURCE_PORT: u16 = 4243;
const ENABLE_PTH_SENSOR: bool = true;
const STANDARD_GRAVITY_MPS2: f64 = 9.80665;
const UDP_TARGET_IPV4: Option<[u8; 4]> = Some([10,86,108,214]);
const INITIAL_LLA_DEG_M: Option<(f64, f64, f64)> = Some((34.7232972,-86.6386501, 500000.0));

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[derive(Clone, Copy, Debug)]
enum GpsFixSource {
    InitialLla,
    Receiver,
}

impl GpsFixSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::InitialLla => "initial_lla",
            Self::Receiver => "gps",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct GpsFix {
    latitude_deg: f64,
    longitude_deg: f64,
    altitude_m: f64,
    ecef_m: Cartesian<f64, ITRF<f64>>,
    ned_m: Cartesian<f64, NED<f64>>,
    satellites: Option<u32>,
    hdop: Option<f32>,
    speed_knots: Option<f32>,
    true_course_deg: Option<f32>,
    source: GpsFixSource,
}

#[derive(Clone, Copy, Debug)]
struct GpsReference {
    latitude_deg: f64,
    longitude_deg: f64,
    ecef_m: Cartesian<f64, ITRF<f64>>,
}

struct GpsDiagnostics {
    total_bytes: u32,
    sentence_count: u32,
    parse_error_count: u32,
    saw_rx: bool,
    satellites: Option<u32>,
    last_sentence: String<LAST_NMEA_SENTENCE_CAPACITY>,
    last_sentence_kind: String<8>,
    last_gga_fix_quality: Option<u8>,
    last_rmc_status: Option<char>,
}

impl GpsDiagnostics {
    fn new() -> Self {
        Self {
            total_bytes: 0,
            sentence_count: 0,
            parse_error_count: 0,
            saw_rx: false,
            satellites: None,
            last_sentence: String::new(),
            last_sentence_kind: String::new(),
            last_gga_fix_quality: None,
            last_rmc_status: None,
        }
    }

    fn record_sentence(&mut self, sentence: &str) {
        self.sentence_count = self.sentence_count.wrapping_add(1);
        self.last_sentence.clear();
        self.last_sentence_kind.clear();
        self.last_gga_fix_quality = None;
        self.last_rmc_status = None;

        for ch in sentence.chars() {
            if self.last_sentence.push(ch).is_err() {
                break;
            }
        }

        if let Some(kind) = sentence
            .split(',')
            .next()
            .map(|kind| kind.trim_start_matches('$'))
        {
            for ch in kind.chars() {
                if self.last_sentence_kind.push(ch).is_err() {
                    break;
                }
            }

            if kind.ends_with("GGA") {
                self.satellites = parse_nmea_u32_field(sentence, 7);
                self.last_gga_fix_quality = sentence
                    .split(',')
                    .nth(6)
                    .and_then(|field| field.parse::<u8>().ok());
            }

            if kind.ends_with("GNS") {
                self.satellites = parse_nmea_u32_field(sentence, 7);
            }

            if kind.ends_with("GSV") {
                if let Some(satellites) = parse_nmea_u32_field(sentence, 3) {
                    self.satellites = Some(satellites);
                }
            }

            if kind.ends_with("RMC") {
                self.last_rmc_status = sentence.split(',').nth(2).and_then(|field| field.chars().next());
            }
        }
    }
}

fn parse_nmea_u32_field(sentence: &str, index: usize) -> Option<u32> {
    sentence
        .split(',')
        .nth(index)
        .and_then(|field| field.split('*').next())
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .and_then(|field| field.parse::<u32>().ok())
}

#[derive(Clone, Copy, Debug)]
struct PthReading {
    pressure_pa: f64,
    temperature_c: f64,
    humidity_percent: f64,
}

impl From<(f64, f64, f64)> for PthReading {
    fn from((pressure_pa, temperature_c, humidity_percent): (f64, f64, f64)) -> Self {
        Self {
            pressure_pa,
            temperature_c,
            humidity_percent,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ImuReading {
    accel_mps2: [f32; 3],
    gyro_dps: [f32; 3],
    mag_ut: [f32; 3],
}

impl From<(f32, f32, f32, f32, f32, f32, f32, f32, f32)> for ImuReading {
    fn from((ax, ay, az, gx, gy, gz, mx, my, mz): (f32, f32, f32, f32, f32, f32, f32, f32, f32)) -> Self {
        Self {
            accel_mps2: [ax, ay, az],
            gyro_dps: [gx, gy, gz],
            mag_ut: [mx, my, mz],
        }
    }
}

struct NavigationFrames {
    icrf_position_m: Cartesian<f64, ICRF<f64>>,
    roll_rad: f64,
    pitch_rad: f64,
    yaw_rad: f64,
}

#[derive(Clone, Copy, Debug)]
struct DeadReckoningState {
    position_ned_m: Cartesian<f64, NED<f64>>,
    velocity_ned_m: Cartesian<f64, NED<f64>>,
    last_update_ms: Option<u64>,
}

impl DeadReckoningState {
    fn new() -> Self {
        Self {
            position_ned_m: Cartesian::new(0.0, 0.0, 0.0),
            velocity_ned_m: Cartesian::new(0.0, 0.0, 0.0),
            last_update_ms: None,
        }
    }

    fn position(&self) -> Cartesian<f64, NED<f64>> {
        self.position_ned_m
    }

    fn update(&mut self, imu: &ImuReading, attitude: &EstimatedAttitude, uptime_ms: u64) {
        let Some(last_update_ms) = self.last_update_ms else {
            self.last_update_ms = Some(uptime_ms);
            return;
        };

        let dt_s = uptime_ms.saturating_sub(last_update_ms) as f64 * 1.0e-3;
        self.last_update_ms = Some(uptime_ms);
        if dt_s <= 0.0 {
            return;
        }

        let accel_body = Cartesian::new(
            imu.accel_mps2[0] as f64,
            imu.accel_mps2[1] as f64,
            imu.accel_mps2[2] as f64,
        );
        let accel_ned = attitude.alignment_body_to_ned * accel_body;
        let linear_accel_ned: Cartesian<f64, NED<f64>> = Cartesian::new(
            accel_ned.x(),
            accel_ned.y(),
            accel_ned.z() - STANDARD_GRAVITY_MPS2,
        );

        let linear_accel_ned: Cartesian<f64, NED<f64>> = if linear_accel_ned.norm() < 0.05 {
            Cartesian::new(0.0, 0.0, 0.0)
        } else {
            linear_accel_ned
        };

        self.position_ned_m = Cartesian::new(
            self.position_ned_m.x() + self.velocity_ned_m.x() * dt_s + 0.5 * linear_accel_ned.x() * dt_s * dt_s,
            self.position_ned_m.y() + self.velocity_ned_m.y() * dt_s + 0.5 * linear_accel_ned.y() * dt_s * dt_s,
            self.position_ned_m.z() + self.velocity_ned_m.z() * dt_s + 0.5 * linear_accel_ned.z() * dt_s * dt_s,
        );
        self.velocity_ned_m = Cartesian::new(
            self.velocity_ned_m.x() + linear_accel_ned.x() * dt_s,
            self.velocity_ned_m.y() + linear_accel_ned.y() * dt_s,
            self.velocity_ned_m.z() + linear_accel_ned.z() * dt_s,
        );
    }
}

#[derive(Clone, Copy, Debug)]
struct EstimatedAttitude {
    roll_rad: f64,
    pitch_rad: f64,
    yaw_rad: f64,
    north_body: Cartesian<f64, Body<f64>>,
    east_body: Cartesian<f64, Body<f64>>,
    down_body: Cartesian<f64, Body<f64>>,
    alignment_body_to_ned: DirectionCosineMatrix<f64, Body<f64>, NED<f64>>,
    alignment_ned_to_body: DirectionCosineMatrix<f64, NED<f64>, Body<f64>>,
    alignment_quaternion: Quaternion<f64, Body<f64>, NED<f64>>,
}

impl EstimatedAttitude {
    fn from_accel_only(imu: &ImuReading) -> Option<Self> {
        let gravity_body: Cartesian<f64, Body<f64>> = Cartesian::new(
            imu.accel_mps2[0] as f64,
            imu.accel_mps2[1] as f64,
            imu.accel_mps2[2] as f64,
        );
        let gravity_norm = gravity_body.norm();
        if gravity_norm <= f64::EPSILON {
            return None;
        }

        let down_body = gravity_body.normalize();

        let reference_axes = [
            Cartesian::new(1.0, 0.0, 0.0),
            Cartesian::new(0.0, 1.0, 0.0),
            Cartesian::new(0.0, 0.0, 1.0),
        ];

        let mut north_body = None;
        for axis in reference_axes {
            let horizontal = axis - down_body * axis.dot(&down_body);
            let horizontal_norm = horizontal.norm();
            if horizontal_norm > 1.0e-9 {
                north_body = Some(horizontal / horizontal_norm);
                break;
            }
        }
        let north_body = north_body?;

        let east_body_raw = down_body.cross(&north_body);
        let east_norm = east_body_raw.norm();
        if east_norm <= f64::EPSILON {
            return None;
        }
        let east_body = east_body_raw / east_norm;
        let north_body = east_body.cross(&down_body).normalize();

        let alignment_body_to_ned = DirectionCosineMatrix::new(
            north_body.x(), north_body.y(), north_body.z(),
            east_body.x(), east_body.y(), east_body.z(),
            down_body.x(), down_body.y(), down_body.z()
        );
        Self::from_dcm(alignment_body_to_ned)
    }

    fn from_accel_mag(imu: &ImuReading) -> Option<Self> {
        let gravity_body: Cartesian<f64, Body<f64>> = Cartesian::new(
            imu.accel_mps2[0] as f64,
            imu.accel_mps2[1] as f64,
            imu.accel_mps2[2] as f64,
        );
        let gravity_norm = gravity_body.norm();
        if gravity_norm <= f64::EPSILON {
            return None;
        }

        let down_body = gravity_body.normalize();
        let magnetometer_body = Cartesian::new(
            imu.mag_ut[0] as f64,
            imu.mag_ut[1] as f64,
            imu.mag_ut[2] as f64,
        );
        let magnetic_vertical = down_body * magnetometer_body.dot(&down_body);
        let magnetic_horizontal = magnetometer_body - magnetic_vertical;
        let magnetic_horizontal_norm = magnetic_horizontal.norm();
        if magnetic_horizontal_norm <= f64::EPSILON {
            return None;
        }

        let north_body = magnetic_horizontal.normalize();
        let east_body_raw = down_body.cross(&north_body);
        let east_norm = east_body_raw.norm();
        if east_norm <= f64::EPSILON {
            return None;
        }
        let east_body = east_body_raw / east_norm;

        let alignment_body_to_ned = DirectionCosineMatrix::new(
            north_body.x(), north_body.y(), north_body.z(),
            east_body.x(), east_body.y(), east_body.z(),
            down_body.x(), down_body.y(), down_body.z()
        );
        Self::from_dcm(alignment_body_to_ned)
    }

    fn from_quaternion(quaternion: Quaternion<f64, Body<f64>, NED<f64>>) -> Option<Self> {
        Self::from_dcm(quaternion.to_dcm())
    }

    fn from_dcm(alignment_body_to_ned: DirectionCosineMatrix<f64, Body<f64>, NED<f64>>) -> Option<Self> {
        let alignment_ned_to_body = alignment_body_to_ned.transpose();
        let alignment_quaternion = Quaternion::try_from(&alignment_body_to_ned).ok()?;
        let euler = Euler::<f64, Body<f64>, NED<f64>>::from(&alignment_body_to_ned);
        let matrix = &alignment_body_to_ned.as_matrix().data;

        Some(Self {
            roll_rad: euler.roll(),
            pitch_rad: euler.pitch(),
            yaw_rad: euler.yaw(),
            north_body: Cartesian::new(matrix[0][0], matrix[0][1], matrix[0][2]),
            east_body: Cartesian::new(matrix[1][0], matrix[1][1], matrix[1][2]),
            down_body: Cartesian::new(matrix[2][0], matrix[2][1], matrix[2][2]),
            alignment_body_to_ned,
            alignment_ned_to_body,
            alignment_quaternion,
        })
    }
}

struct AttitudeTracker {
    attitude: Option<EstimatedAttitude>,
    last_update_ms: Option<u64>,
}

impl AttitudeTracker {
    fn new() -> Self {
        Self {
            attitude: None,
            last_update_ms: None,
        }
    }

    fn attitude(&self) -> Option<EstimatedAttitude> {
        self.attitude
    }

    fn update(&mut self, imu: &ImuReading, uptime_ms: u64) -> Option<EstimatedAttitude> {
        if self.attitude.is_none() {
            self.attitude = EstimatedAttitude::from_accel_mag(imu)
                .or_else(|| EstimatedAttitude::from_accel_only(imu));
            self.last_update_ms = Some(uptime_ms);
            return self.attitude;
        }

        let dt_s = self
            .last_update_ms
            .map(|last| uptime_ms.saturating_sub(last) as f64 * 1.0e-3)
            .unwrap_or(0.0);
        self.last_update_ms = Some(uptime_ms);

        if dt_s <= 0.0 {
            return self.attitude;
        }

        let Some(attitude) = self.attitude else {
            return None;
        };

        let angular_velocity_body: Cartesian<f64, Body<f64>> = Cartesian::new(
            (imu.gyro_dps[0] as f64).to_radians(),
            (imu.gyro_dps[1] as f64).to_radians(),
            (imu.gyro_dps[2] as f64).to_radians(),
        );
        let ned_to_body = attitude.alignment_quaternion.inverse();
        let delta_quaternion = Quaternion::<f64, Body<f64>, Body<f64>>::from_angular_velocity(
            angular_velocity_body.data,
            dt_s,
        );
        let updated_quaternion = (&delta_quaternion * &ned_to_body).inverse();

        self.attitude = EstimatedAttitude::from_quaternion(updated_quaternion)
            .or_else(|| EstimatedAttitude::from_accel_mag(imu))
            .or_else(|| EstimatedAttitude::from_accel_only(imu))
            .or(self.attitude);
        self.attitude
    }
}

impl AttitudeState<f64> for EstimatedAttitude {
    fn north_body(&self) -> Cartesian<f64, Body<f64>> {
        self.north_body
    }

    fn east_body(&self) -> Cartesian<f64, Body<f64>> {
        self.east_body
    }

    fn down_body(&self) -> Cartesian<f64, Body<f64>> {
        self.down_body
    }

    fn alignment_body_to_ned(&self) -> DirectionCosineMatrix<f64, Body<f64>, NED<f64>> {
        self.alignment_body_to_ned
    }

    fn alignment_ned_to_body(&self) -> DirectionCosineMatrix<f64, NED<f64>, Body<f64>> {
        self.alignment_ned_to_body
    }

    fn alignment_quaternion(&self) -> Quaternion<f64, Body<f64>, NED<f64>> {
        self.alignment_quaternion
    }
}

#[derive(Clone, Copy, Debug)]
struct NavigationHotStart {
    initial_fix: Option<GeodeticHotStart<f64>>,
    navigator: Option<AbsoluteNavigator<f64>>,
}

impl NavigationHotStart {
    fn new() -> Self {
        Self {
            initial_fix: None,
            navigator: None,
        }
    }

    fn navigator(&self) -> Option<AbsoluteNavigator<f64>> {
        self.navigator
    }

    fn initial_fix(&self) -> Option<GeodeticHotStart<f64>> {
        self.initial_fix
    }

    fn maybe_initialize(&mut self, fix: &GpsFix, attitude: &EstimatedAttitude, elapsed_s: f64) -> bool {
        if self.navigator.is_some() {
            return false;
        }

        let hot_start = GeodeticHotStart::new(
            fix.latitude_deg.to_radians(),
            fix.longitude_deg.to_radians(),
            fix.altitude_m,
            elapsed_s,
        );

        self.navigator = Some(AbsoluteNavigator::hot_start_from_attitude_state(
            attitude,
            hot_start,
        ));
        self.initial_fix = Some(hot_start);
        true
    }
}

fn build_navigation_frames(fix: &GpsFix, attitude: &EstimatedAttitude, elapsed_s: f64) -> NavigationFrames {
    let earth_itrf = ITRF::<f64>::default();
    let itrf_to_icrf_dcm = itrf_to_icrf(elapsed_s, earth_itrf.angular_velocity());
    let icrf_position_m = itrf_to_icrf_dcm * fix.ecef_m;

    NavigationFrames {
        icrf_position_m,
        roll_rad: attitude.roll_rad,
        pitch_rad: attitude.pitch_rad,
        yaw_rad: attitude.yaw_rad,
    }
}

fn dead_reckoned_fix(
    base_fix: GpsFix,
    reference: GpsReference,
    dead_reckoning: &DeadReckoningState,
) -> GpsFix {
    let ned_m = dead_reckoning.position();
    let ned_to_itrf = itrf_to_ned(
        reference.latitude_deg.to_radians(),
        reference.longitude_deg.to_radians(),
    )
    .transpose();
    let delta_itrf_m = ned_to_itrf * ned_m;
    let ecef_m = Cartesian::new(
        reference.ecef_m.x() + delta_itrf_m.x(),
        reference.ecef_m.y() + delta_itrf_m.y(),
        reference.ecef_m.z() + delta_itrf_m.z(),
    );

    GpsFix { ecef_m, ned_m, ..base_fix }
}

struct GpsState {
    parser: Nmea,
    sentence: String<NMEA_SENTENCE_CAPACITY>,
    reference: Option<GpsReference>,
    last_fix: Option<GpsFix>,
    diagnostics: GpsDiagnostics,
}

impl GpsState {
    fn new(initial_lla_deg_m: Option<(f64, f64, f64)>) -> Self {
        let mut state = Self {
            parser: Nmea::default(),
            sentence: String::new(),
            reference: None,
            last_fix: None,
            diagnostics: GpsDiagnostics::new(),
        };

        if let Some((latitude_deg, longitude_deg, altitude_m)) = initial_lla_deg_m {
            let ecef_m = geocentric_to_ecef(
                latitude_deg.to_radians(),
                longitude_deg.to_radians(),
                altitude_m,
            );

            state.reference = Some(GpsReference {
                latitude_deg,
                longitude_deg,
                ecef_m,
            });
            state.last_fix = Some(GpsFix {
                latitude_deg,
                longitude_deg,
                altitude_m,
                ecef_m,
                ned_m: Cartesian::new(0.0, 0.0, 0.0),
                satellites: None,
                hdop: None,
                speed_knots: None,
                true_course_deg: None,
                source: GpsFixSource::InitialLla,
            });
        }

        state
    }

    fn last_fix(&self) -> Option<GpsFix> {
        self.last_fix
    }

    fn reference(&self) -> Option<GpsReference> {
        self.reference
    }

    fn diagnostics(&self) -> &GpsDiagnostics {
        &self.diagnostics
    }

    fn process_sentence(&mut self, sentence: &str) -> Result<Option<GpsFix>, ()> {
        if self.parser.parse(sentence).is_err() {
            return Err(());
        }

        self.diagnostics.satellites = self.parser.fix_satellites().or(self.diagnostics.satellites);

        let Some(latitude_deg) = self.parser.latitude() else {
            return Ok(None);
        };
        let Some(longitude_deg) = self.parser.longitude() else {
            return Ok(None);
        };
        let altitude_m = self.parser.altitude().unwrap_or_default() as f64;
        let ecef_m = geocentric_to_ecef(latitude_deg.to_radians(), longitude_deg.to_radians(), altitude_m);

        let reference_fix = self.reference.get_or_insert(GpsReference {
            latitude_deg,
            longitude_deg,
            ecef_m,
        });

        let delta_ecef_m = ecef_m - reference_fix.ecef_m;
        let ned_m = itrf_to_ned(
            reference_fix.latitude_deg.to_radians(),
            reference_fix.longitude_deg.to_radians(),
        ) * delta_ecef_m;

        Ok(Some(GpsFix {
            latitude_deg,
            longitude_deg,
            altitude_m,
            ecef_m,
            ned_m,
            satellites: self.parser.fix_satellites().or(self.diagnostics.satellites),
            hdop: self.parser.hdop(),
            speed_knots: self.parser.speed_over_ground,
            true_course_deg: self.parser.true_course,
            source: GpsFixSource::Receiver,
        }))
    }

    fn poll_uart(&mut self, uart: &mut GpsUart) {
        let (_, rx) = uart.split_ref();

        for _ in 0..64 {
            match embedded_hal_0_2::serial::Read::read(rx) {
                Ok(byte) => {
                    self.diagnostics.saw_rx = true;
                    self.diagnostics.total_bytes = self.diagnostics.total_bytes.wrapping_add(1);

                    match byte {
                        b'\r' => {}
                        b'\n' => {
                            if !self.sentence.is_empty() {
                                let mut completed_sentence = String::<NMEA_SENTENCE_CAPACITY>::new();

                                for ch in self.sentence.chars() {
                                    if completed_sentence.push(ch).is_err() {
                                        break;
                                    }
                                }

                                self.diagnostics.record_sentence(completed_sentence.as_str());

                                match self.process_sentence(completed_sentence.as_str()) {
                                    Ok(Some(fix)) => self.last_fix = Some(fix),
                                    Ok(None) => {}
                                    Err(()) => {
                                        self.diagnostics.parse_error_count =
                                            self.diagnostics.parse_error_count.wrapping_add(1);
                                    }
                                }
                            }

                            self.sentence.clear();
                        }
                        _ => {
                            if self.sentence.push(byte as char).is_err() {
                                self.sentence.clear();
                            }
                        }
                    }
                }
                Err(nb::Error::WouldBlock) => break,
                Err(nb::Error::Other(_)) => break,
            }
        }
    }
}

#[derive(Clone, Copy)]
struct SharedI2cDevice<'a, BUS> {
    bus: &'a RefCell<BUS>,
}

impl<'a, BUS> SharedI2cDevice<'a, BUS> {
    fn new(bus: &'a RefCell<BUS>) -> Self {
        Self { bus }
    }
}

impl<BUS, E> I2cWrite for SharedI2cDevice<'_, BUS>
where
    BUS: I2cWrite<Error = E>,
{
    type Error = E;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.bus.borrow_mut().write(addr, bytes)
    }
}

impl<BUS, E> I2cWriteRead for SharedI2cDevice<'_, BUS>
where
    BUS: I2cWriteRead<Error = E>,
{
    type Error = E;

    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.bus.borrow_mut().write_read(addr, bytes, buffer)
    }
}

impl<BUS, E> I2cRead for SharedI2cDevice<'_, BUS>
where
    BUS: I2cRead<Error = E>,
{
    type Error = E;

    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.bus.borrow_mut().read(addr, buffer)
    }
}

enum IcmDevice<'a, BUS, E> {
    Addr69(ICMI2C<SharedI2cDevice<'a, BUS>, E, ICM20948_CHIP_ADR>),
    Addr68(ICMI2C<SharedI2cDevice<'a, BUS>, E, ICM20948_CHIP_ADR_ALT>),
}

impl<'a, BUS, E> IcmDevice<'a, BUS, E>
where
    BUS: I2cRead<Error = E> + I2cWrite<Error = E> + I2cWriteRead<Error = E>,
{
    fn init(delay: &mut Delay, i2c: &mut SharedI2cDevice<'a, BUS>) -> Result<Self, ICMError<E>> {
        let mut icm = ICMI2C::<SharedI2cDevice<'a, BUS>, E, ICM20948_CHIP_ADR>::new(i2c)
            .map_err(ICMError::Raw)?;
        if icm.init(i2c, delay).is_ok() {
            return Ok(Self::Addr69(icm));
        }

        let mut icm = ICMI2C::<SharedI2cDevice<'a, BUS>, E, ICM20948_CHIP_ADR_ALT>::new(i2c)
            .map_err(ICMError::Raw)?;
        icm.init(i2c, delay)?;
        Ok(Self::Addr68(icm))
    }

    fn read_scaled_with_mag(&self, i2c: &mut SharedI2cDevice<'a, BUS>) -> Result<(f32, f32, f32, f32, f32, f32, f32, f32, f32), ICMError<E>> {
        match self {
            Self::Addr69(icm) => {
                let raw = icm.get_values_accel_gyro(i2c)?;
                let scaled = icm.scale_raw_accel_gyro(raw);
                Ok((
                    scaled.0, scaled.1, scaled.2, scaled.3, scaled.4, scaled.5, 0.0, 0.0, 0.0,
                ))
            }
            Self::Addr68(icm) => {
                let raw = icm.get_values_accel_gyro(i2c)?;
                let scaled = icm.scale_raw_accel_gyro(raw);
                Ok((
                    scaled.0, scaled.1, scaled.2, scaled.3, scaled.4, scaled.5, 0.0, 0.0, 0.0,
                ))
            }
        }
    }

    fn address(&self) -> u8 {
        match self {
            Self::Addr69(_) => ICM20948_CHIP_ADR,
            Self::Addr68(_) => ICM20948_CHIP_ADR_ALT,
        }
    }

    fn wake_sensors(&self, i2c: &mut SharedI2cDevice<'a, BUS>) -> Result<(), E> {
        i2c.write(self.address(), &[ICM_PWR_MGMT_1_REG, 0x01])?;
        i2c.write(self.address(), &[ICM_PWR_MGMT_2_REG, 0x00])?;
        Ok(())
    }

    fn configure_magnetometer(&self, i2c: &mut SharedI2cDevice<'a, BUS>) -> Result<(), E> {
        i2c.write(self.address(), &[ICM_INT_PIN_CFG_REG, ICM_INT_PIN_CFG_BYPASS_ENABLE])?;
        i2c.write(AK09916_I2C_ADDR, &[AK09916_CNTL2_REG, AK09916_MODE_CONTINUOUS_100HZ])?;
        Ok(())
    }

    fn read_mag_ut(i2c: &mut SharedI2cDevice<'a, BUS>) -> Result<[f32; 3], E> {
        let mut status = [0_u8; 1];
        i2c.write_read(AK09916_I2C_ADDR, &[AK09916_ST1_REG], &mut status)?;
        if status[0] & 0x01 == 0 {
            return Ok([0.0, 0.0, 0.0]);
        }

        let mut buffer = [0_u8; 8];
        i2c.write_read(AK09916_I2C_ADDR, &[AK09916_HXL_REG], &mut buffer)?;
        let x = i16::from_le_bytes([buffer[0], buffer[1]]) as f32 * AK09916_UT_PER_LSB;
        let y = i16::from_le_bytes([buffer[2], buffer[3]]) as f32 * AK09916_UT_PER_LSB;
        let z = i16::from_le_bytes([buffer[4], buffer[5]]) as f32 * AK09916_UT_PER_LSB;
        Ok([x, y, z])
    }

    fn read_reg(&self, i2c: &mut SharedI2cDevice<'a, BUS>, reg: u8) -> Result<u8, E> {
        let mut value = [0_u8; 1];
        i2c.write_read(self.address(), &[reg], &mut value)?;
        Ok(value[0])
    }
}

type GpsUart = Uart<'static, Blocking>;

fn recover_i2c_bus(
    scl_pin: embassy_rp::Peri<'_, impl embassy_rp::gpio::Pin>,
    sda_pin: embassy_rp::Peri<'_, impl embassy_rp::gpio::Pin>,
) {
    let mut scl = OutputOpenDrain::new(scl_pin, Level::High);
    scl.set_pullup(true);

    let sda = Input::new(sda_pin, Pull::Up);
    let mut sda_high = sda.is_high();

    for _ in 0..16 {
        if sda_high {
            break;
        }

        scl.set_low();
        block_for(Duration::from_micros(10));
        scl.set_high();
        block_for(Duration::from_micros(10));
        sda_high = sda.is_high();
    }

    info!("i2c bus recovery: sda_high={}", sda_high);
}

fn build_sensor_snapshot(
    gps: &GpsState,
    hot_start: &mut NavigationHotStart,
    display_fix: Option<GpsFix>,
    attitude: Option<EstimatedAttitude>,
    pth: Option<PthReading>,
    imu: Option<ImuReading>,
    imu_read_failures: u32,
    elapsed_s: f64,
    uptime_ms: u64,
) -> String<STATE_MESSAGE_CAPACITY> {
    let mut message = String::<STATE_MESSAGE_CAPACITY>::new();
    let fix = display_fix.or_else(|| gps.last_fix());

    match (fix, imu, attitude, pth) {
        (Some(fix), Some(imu), Some(attitude), pth) => {
            let hot_started = hot_start.maybe_initialize(&fix, &attitude, elapsed_s);
            let frames = build_navigation_frames(&fix, &attitude, elapsed_s);
            let hot_start_fix = hot_start.initial_fix();
            let hot_start_nav = hot_start.navigator();
            let _ = write!(
                message,
                "t_ms={} lat={:.6} lon={:.6} alt_m={:.1} src={} sats={:?} hdop={:?} sog={:?} cog={:?} ecef=[{:.1},{:.1},{:.1}] ned=[{:.1},{:.1},{:.1}] icrf=[{:.1},{:.1},{:.1}] rpy_deg=[{:.1},{:.1},{:.1}] north_body=[{:.3},{:.3},{:.3}] east_body=[{:.3},{:.3},{:.3}] down_body=[{:.3},{:.3},{:.3}] imu_errs={} hot={} lat0={:?} lon0={:?} alt0={:?} icrf0={:?}",
                uptime_ms,
                fix.latitude_deg,
                fix.longitude_deg,
                fix.altitude_m,
                fix.source.as_str(),
                fix.satellites,
                fix.hdop,
                fix.speed_knots,
                fix.true_course_deg,
                fix.ecef_m.x(),
                fix.ecef_m.y(),
                fix.ecef_m.z(),
                fix.ned_m.x(),
                fix.ned_m.y(),
                fix.ned_m.z(),
                frames.icrf_position_m.x(),
                frames.icrf_position_m.y(),
                frames.icrf_position_m.z(),
                frames.roll_rad.to_degrees(),
                frames.pitch_rad.to_degrees(),
                frames.yaw_rad.to_degrees(),
                attitude.north_body.x(),
                attitude.north_body.y(),
                attitude.north_body.z(),
                attitude.east_body.x(),
                attitude.east_body.y(),
                attitude.east_body.z(),
                attitude.down_body.x(),
                attitude.down_body.y(),
                attitude.down_body.z(),
                imu_read_failures,
                hot_started || hot_start_nav.is_some(),
                hot_start_fix.map(|fix| fix.latitude_rad.to_degrees()),
                hot_start_fix.map(|fix| fix.longitude_rad.to_degrees()),
                hot_start_fix.map(|fix| fix.altitude_m),
                hot_start_nav.map(|nav| (nav.position_icrf_m.x(), nav.position_icrf_m.y(), nav.position_icrf_m.z())),
            );

            if let Some(pth) = pth {
                let _ = write!(
                    message,
                    " pth=[{:.2},{:.2},{:.2}]",
                    pth.pressure_pa,
                    pth.temperature_c,
                    pth.humidity_percent,
                );
            } else {
                let _ = write!(message, " pth=unavailable");
            }

            let _ = write!(
                message,
                " imu_accel=[{:.2},{:.2},{:.2}] imu_gyro=[{:.2},{:.2},{:.2}] imu_mag=[{:.2},{:.2},{:.2}]",
                imu.accel_mps2[0],
                imu.accel_mps2[1],
                imu.accel_mps2[2],
                imu.gyro_dps[0],
                imu.gyro_dps[1],
                imu.gyro_dps[2],
                imu.mag_ut[0],
                imu.mag_ut[1],
                imu.mag_ut[2],
            );
        }
        (Some(fix), Some(imu), None, pth) => {
            let _ = write!(
                message,
                "t_ms={} lat={:.6} lon={:.6} alt_m={:.1} src={} ecef=[{:.1},{:.1},{:.1}] ned=[{:.1},{:.1},{:.1}] imu_errs={} hot=false att=uninitialized",
                uptime_ms,
                fix.latitude_deg,
                fix.longitude_deg,
                fix.altitude_m,
                fix.source.as_str(),
                fix.ecef_m.x(),
                fix.ecef_m.y(),
                fix.ecef_m.z(),
                fix.ned_m.x(),
                fix.ned_m.y(),
                fix.ned_m.z(),
                imu_read_failures,
            );

            if let Some(pth) = pth {
                let _ = write!(
                    message,
                    " pth=[{:.2},{:.2},{:.2}]",
                    pth.pressure_pa,
                    pth.temperature_c,
                    pth.humidity_percent,
                );
            } else {
                let _ = write!(message, " pth=unavailable");
            }

            let _ = write!(
                message,
                " imu_accel=[{:.2},{:.2},{:.2}] imu_gyro=[{:.2},{:.2},{:.2}] imu_mag=[{:.2},{:.2},{:.2}]",
                imu.accel_mps2[0],
                imu.accel_mps2[1],
                imu.accel_mps2[2],
                imu.gyro_dps[0],
                imu.gyro_dps[1],
                imu.gyro_dps[2],
                imu.mag_ut[0],
                imu.mag_ut[1],
                imu.mag_ut[2],
            );
        }
        (None, Some(imu), _, pth) => {
            let diagnostics = gps.diagnostics();
            let _ = write!(
                message,
                "t_ms={} gps_wait=1 rx={} bytes={} sentences={} parse_err={} sats={:?} kind={} gga_fix={:?} rmc_status={:?} imu_errs={} last='{}'",
                uptime_ms,
                diagnostics.saw_rx,
                diagnostics.total_bytes,
                diagnostics.sentence_count,
                diagnostics.parse_error_count,
                diagnostics.satellites,
                diagnostics.last_sentence_kind.as_str(),
                diagnostics.last_gga_fix_quality,
                diagnostics.last_rmc_status,
                imu_read_failures,
                diagnostics.last_sentence.as_str(),
            );

            if let Some(pth) = pth {
                let _ = write!(
                    message,
                    " pth=[{:.2},{:.2},{:.2}]",
                    pth.pressure_pa,
                    pth.temperature_c,
                    pth.humidity_percent,
                );
            } else {
                let _ = write!(message, " pth=unavailable");
            }

            let _ = write!(
                message,
                " imu_accel=[{:.2},{:.2},{:.2}] imu_gyro=[{:.2},{:.2},{:.2}] imu_mag=[{:.2},{:.2},{:.2}]",
                imu.accel_mps2[0],
                imu.accel_mps2[1],
                imu.accel_mps2[2],
                imu.gyro_dps[0],
                imu.gyro_dps[1],
                imu.gyro_dps[2],
                imu.mag_ut[0],
                imu.mag_ut[1],
                imu.mag_ut[2],
            );
        }
        (Some(fix), None, _, pth) => {
            let _ = write!(
                message,
                "t_ms={} lat={:.6} lon={:.6} alt_m={:.1} src={} ecef=[{:.1},{:.1},{:.1}] ned=[{:.1},{:.1},{:.1}] imu_errs={} hot=false",
                uptime_ms,
                fix.latitude_deg,
                fix.longitude_deg,
                fix.altitude_m,
                fix.source.as_str(),
                fix.ecef_m.x(),
                fix.ecef_m.y(),
                fix.ecef_m.z(),
                fix.ned_m.x(),
                fix.ned_m.y(),
                fix.ned_m.z(),
                imu_read_failures,
            );

            if let Some(pth) = pth {
                let _ = write!(
                    message,
                    " pth=[{:.2},{:.2},{:.2}]",
                    pth.pressure_pa,
                    pth.temperature_c,
                    pth.humidity_percent,
                );
            } else {
                let _ = write!(message, " pth=unavailable");
            }

            let _ = write!(message, " imu=unavailable");
        }
        (None, None, _, pth) => {
            let diagnostics = gps.diagnostics();
            let _ = write!(
                message,
                "t_ms={} gps_wait=1 rx={} bytes={} sentences={} parse_err={} sats={:?} kind={} gga_fix={:?} rmc_status={:?} imu_errs={} last='{}'",
                uptime_ms,
                diagnostics.saw_rx,
                diagnostics.total_bytes,
                diagnostics.sentence_count,
                diagnostics.parse_error_count,
                diagnostics.satellites,
                diagnostics.last_sentence_kind.as_str(),
                diagnostics.last_gga_fix_quality,
                diagnostics.last_rmc_status,
                imu_read_failures,
                diagnostics.last_sentence.as_str(),
            );

            if let Some(pth) = pth {
                let _ = write!(
                    message,
                    " pth=[{:.2},{:.2},{:.2}]",
                    pth.pressure_pa,
                    pth.temperature_c,
                    pth.humidity_percent,
                );
            } else {
                let _ = write!(message, " pth=unavailable");
            }

            let _ = write!(message, " imu=unavailable");
        }
    }

    message
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("ins-test bring-up starting with wifi");

    let p = embassy_rp::init(Default::default());
    let mut delay = Delay;

    let mut i2c_scl = p.PIN_21;
    let mut i2c_sda = p.PIN_20;
    info!("recovering i2c bus on gp20/gp21");
    recover_i2c_bus(i2c_scl.reborrow(), i2c_sda.reborrow());

    let mut i2c_config = i2c::Config::default();
    i2c_config.frequency = 400_000;
    let i2c = I2c::new_blocking(p.I2C0, i2c_scl, i2c_sda, i2c_config);

    let mut uart_config = uart::Config::default();
    uart_config.baudrate = GPS_BAUD_HZ;
    let mut gps_uart = Uart::new_blocking(p.UART0, p.PIN_12, p.PIN_13, uart_config);

    let fw = aligned_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = aligned_bytes!("../cyw43-firmware/43439A0_clm.bin");
    let nvram = aligned_bytes!("../cyw43-firmware/nvram_rp2040.bin");

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        RM2_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        dma::Channel::new(p.DMA_CH0, Irqs),
    );

    static WIFI_STATE: StaticCell<cyw43::State> = StaticCell::new();
    let wifi_state = WIFI_STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(wifi_state, pwr, spi, fw, nvram).await;
    match cyw43_task(runner) {
        Ok(token) => spawner.spawn(token),
        Err(_) => panic!("failed to spawn cyw43 task"),
    }

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;
    control.gpio_set(0, false).await;

    let mut hostname = String::<32>::new();
    let _ = hostname.push_str(WIFI_HOSTNAME);
    let mut dhcp_config = DhcpConfig::default();
    dhcp_config.hostname = Some(hostname);
    let net_config = NetConfig::dhcpv4(dhcp_config);

    static NET_RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        net_config,
        NET_RESOURCES.init(StackResources::new()),
        0x494E_532D_5445_5354,
    );
    match net_task(runner) {
        Ok(token) => spawner.spawn(token),
        Err(_) => panic!("failed to spawn network task"),
    }

    if WIFI_NETWORK.is_empty() {
        warn!("set WIFI_NETWORK and WIFI_PASSWORD in main_wifi.rs to join an AP");
        loop {
            control.gpio_set(0, true).await;
            Timer::after(Duration::from_millis(200)).await;
            control.gpio_set(0, false).await;
            Timer::after(Duration::from_secs(2)).await;
        }
    }

    loop {
        match control
            .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
            .await
        {
            Ok(()) => break,
            Err(_) => {
                warn!("wifi join failed, retrying");
                Timer::after(Duration::from_secs(1)).await;
            }
        }
    }

    info!("wifi association complete");
    info!("waiting for DHCP hostname registration: {}", WIFI_HOSTNAME);
    stack.wait_link_up().await;
    stack.wait_config_up().await;

    let lease = stack.config_v4().expect("dhcp config after wait_config_up");
    let local_addr = lease.address.address();
    let broadcast_addr = Ipv4Address::from_bits(local_addr.to_bits() | !lease.address.netmask().to_bits());
    let udp_target_addr = UDP_TARGET_IPV4
        .map(|octets| Ipv4Address::new(octets[0], octets[1], octets[2], octets[3]))
        .unwrap_or(broadcast_addr);

    info!("dhcp address: {:?}", local_addr);
    info!("udp stream target: {:?}:{}", udp_target_addr, UDP_STREAM_PORT);

    let mut udp_rx_meta = [PacketMetadata::EMPTY; 4];
    let mut udp_tx_meta = [PacketMetadata::EMPTY; 4];
    let mut udp_rx_buffer = [0u8; 512];
    let mut udp_tx_buffer = [0u8; 2048];
    let mut udp_socket = UdpSocket::new(
        stack,
        &mut udp_rx_meta,
        &mut udp_rx_buffer,
        &mut udp_tx_meta,
        &mut udp_tx_buffer,
    );
    udp_socket.bind(UDP_SOURCE_PORT).unwrap();
    info!("udp socket bound on {}", UDP_SOURCE_PORT);

    let i2c_bus = RefCell::new(i2c);

    let mut ms8607 = if ENABLE_PTH_SENSOR {
        info!("starting ms8607 probe on gp20/gp21");
        let sensor_bus = SharedI2cDevice::new(&i2c_bus);
        let mut sensor = MS8607::new(sensor_bus);
        match sensor.begin(&mut delay) {
            Ok(()) => info!("ms8607 ready on gp20/gp21"),
            Err(_) => warn!("ms8607 init failed"),
        }
        Some(sensor)
    } else {
        warn!("ms8607 probe disabled in wifi mode");
        None
    };

    info!("starting icm20948 probe");
    let icm = {
        let mut sensor_bus = SharedI2cDevice::new(&i2c_bus);
        match IcmDevice::init(&mut delay, &mut sensor_bus) {
            Ok(device) => {
                if device.wake_sensors(&mut sensor_bus).is_err() {
                    warn!("icm20948 wake sequence failed");
                }
                let who_am_i = device.read_reg(&mut sensor_bus, ICM_WHO_AM_I_REG).ok();
                let pwr_mgmt_1 = device.read_reg(&mut sensor_bus, ICM_PWR_MGMT_1_REG).ok();
                let pwr_mgmt_2 = device.read_reg(&mut sensor_bus, ICM_PWR_MGMT_2_REG).ok();

                info!("icm20948 ready at {}", device.address());
                info!(
                    "icm20948 whoami={:?} pwr_mgmt_1={:?} pwr_mgmt_2={:?}",
                    who_am_i,
                    pwr_mgmt_1,
                    pwr_mgmt_2,
                );
                if device.read_scaled_with_mag(&mut sensor_bus).is_err() {
                    panic!("icm20948 detected but accel/gyro read failed during startup");
                }
                info!("icm20948 startup read verified");
                device
            }
            Err(_) => {
                panic!("icm20948 init failed on 0x69 and 0x68");
            }
        }
    };

    info!("gps uart ready on gp12/gp13 at {} baud", GPS_BAUD_HZ);
    info!("entering sensor/udp main loop");

    if INITIAL_LLA_DEG_M.is_some() {
        info!("initial LLA fallback enabled");
    }

    let mut gps = GpsState::new(INITIAL_LLA_DEG_M);
    let mut hot_start = NavigationHotStart::new();
    let mut attitude_tracker = AttitudeTracker::new();
    let mut dead_reckoning = DeadReckoningState::new();
    let mut last_imu = None;
    let mut imu_read_failures = 0_u32;
    let mut poll_counter = 0_u32;
    let mut uptime_ms = 0_u64;

    loop {
        gps.poll_uart(&mut gps_uart);

        let mut sensor_bus = SharedI2cDevice::new(&i2c_bus);
        match icm.read_scaled_with_mag(&mut sensor_bus).map(ImuReading::from) {
            Ok(imu) => {
                let attitude = attitude_tracker.update(&imu, uptime_ms);
                if let Some(attitude) = attitude {
                    dead_reckoning.update(&imu, &attitude, uptime_ms);
                }
                last_imu = Some(imu);
            }
            Err(_) => {
                imu_read_failures = imu_read_failures.wrapping_add(1);
            }
        }

        poll_counter = poll_counter.wrapping_add(1);
        if poll_counter >= SENSOR_POLL_DIVIDER {
            poll_counter = 0;

            let pth = ms8607
                .as_mut()
                .and_then(|sensor| sensor.get_measurements(&mut delay).ok().map(PthReading::from));
            let imu = last_imu;
            let display_fix = gps.last_fix().map(|fix| {
                if matches!(fix.source, GpsFixSource::InitialLla) {
                    gps.reference()
                        .map(|reference| dead_reckoned_fix(fix, reference, &dead_reckoning))
                        .unwrap_or(fix)
                } else {
                    fix
                }
            });

            let snapshot = build_sensor_snapshot(
                &gps,
                &mut hot_start,
                display_fix,
                attitude_tracker.attitude(),
                pth,
                imu,
                imu_read_failures,
                uptime_ms as f64 * 1.0e-3,
                uptime_ms,
            );
            info!("{}", snapshot.as_str());

            if let Err(err) = udp_socket
                .send_to(snapshot.as_bytes(), (udp_target_addr, UDP_STREAM_PORT))
                .await
            {
                warn!("udp stream failed: {:?}", err);
            } else {
                control.gpio_set(0, true).await;
                Timer::after(Duration::from_millis(10)).await;
                control.gpio_set(0, false).await;
            }
        }

        Timer::after(Duration::from_millis(20)).await;
        uptime_ms = uptime_ms.wrapping_add(20);
    }
}
