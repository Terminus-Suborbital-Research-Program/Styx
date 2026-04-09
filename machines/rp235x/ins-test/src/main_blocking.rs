//! Simple INS bring-up for GPS + IMU + pressure/temperature/humidity.

use core::cell::RefCell;

use aether_proprietary::navigation::{AbsoluteNavigator, AttitudeState, GeodeticHotStart};
use aether_models::{
    attitude::{DirectionCosineMatrix, Quaternion},
    coordinate::Cartesian,
    real::Real,
    reference_frame::{
        transforms::{body_to_ned, itrf_to_icrf, itrf_to_ned},
        Body, ICRF, ITRF, NED, RotatingFrame,
    },
    terrestrial::wgs84::transforms::geocentric_to_ecef,
};
use cortex_m::delay::Delay;
use embedded_hal_0_2::blocking::i2c::{Read as I2cRead, Write as I2cWrite, WriteRead as I2cWriteRead};
use hal::{
    fugit::RateExtU32,
    gpio::{FunctionUart, Pin},
    uart::{DataBits, StopBits, UartConfig, UartPeripheral},
    Clock,
};
use heapless::String;
use icm20948::{ICMError, ICMI2C, ICM20948_CHIP_ADR, ICM20948_CHIP_ADR_ALT};
use ms8607::MS8607;
use nmea::Nmea;
use panic_halt as _;
use rp235x_hal as hal;
use rtt_target::ChannelMode::NoBlockSkip;
use rtt_target::{rprintln, rtt_init, set_print_channel};

#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;
const GPS_BAUD_HZ: u32 = 9_600;
const SENSOR_POLL_DIVIDER: u32 = 25;
const NMEA_SENTENCE_CAPACITY: usize = 128;
const LAST_NMEA_SENTENCE_CAPACITY: usize = 96;
const ICM_WHO_AM_I_REG: u8 = 0x00;
const ICM_PWR_MGMT_1_REG: u8 = 0x06;
const ICM_PWR_MGMT_2_REG: u8 = 0x07;

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
}

impl From<(f32, f32, f32, f32, f32, f32)> for ImuReading {
    fn from((ax, ay, az, gx, gy, gz): (f32, f32, f32, f32, f32, f32)) -> Self {
        Self {
            accel_mps2: [ax, ay, az],
            gyro_dps: [gx, gy, gz],
        }
    }
}

struct NavigationFrames {
    body_to_ned: DirectionCosineMatrix<f64, Body<f64>, NED<f64>>,
    ned_to_itrf: DirectionCosineMatrix<f64, NED<f64>, ITRF<f64>>,
    itrf_to_icrf: DirectionCosineMatrix<f64, ITRF<f64>, ICRF<f64>>,
    icrf_position_m: Cartesian<f64, ICRF<f64>>,
    roll_rad: f64,
    pitch_rad: f64,
    yaw_rad: f64,
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
    fn from_snapshot(imu: &ImuReading, fix: &GpsFix) -> Option<Self> {
        let (roll_rad, pitch_rad, yaw_rad) = estimate_body_euler_rad(imu, fix);
        let alignment_body_to_ned = body_to_ned(roll_rad, pitch_rad, yaw_rad);
        let alignment_ned_to_body = alignment_body_to_ned.transpose();
        let alignment_quaternion = Quaternion::try_from(&alignment_body_to_ned).ok()?;
        let matrix = &alignment_body_to_ned.as_matrix().data;

        Some(Self {
            roll_rad,
            pitch_rad,
            yaw_rad,
            north_body: Cartesian::new(matrix[0][0], matrix[0][1], matrix[0][2]),
            east_body: Cartesian::new(matrix[1][0], matrix[1][1], matrix[1][2]),
            down_body: Cartesian::new(matrix[2][0], matrix[2][1], matrix[2][2]),
            alignment_body_to_ned,
            alignment_ned_to_body,
            alignment_quaternion,
        })
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

    fn maybe_initialize(&mut self, fix: &GpsFix, imu: &ImuReading, elapsed_s: f64) -> bool {
        if self.navigator.is_some() {
            return false;
        }

        let Some(attitude) = EstimatedAttitude::from_snapshot(imu, fix) else {
            return false;
        };

        let hot_start = GeodeticHotStart::new(
            fix.latitude_deg.to_radians(),
            fix.longitude_deg.to_radians(),
            fix.altitude_m,
            elapsed_s,
        );

        self.navigator = Some(AbsoluteNavigator::hot_start_from_attitude_state(
            &attitude,
            hot_start,
        ));
        self.initial_fix = Some(hot_start);
        true
    }
}

fn estimate_body_euler_rad(imu: &ImuReading, fix: &GpsFix) -> (f64, f64, f64) {
    let ax = imu.accel_mps2[0] as f64;
    let ay = imu.accel_mps2[1] as f64;
    let az = imu.accel_mps2[2] as f64;

    let roll_rad = ay.atan2(az);
    let pitch_rad = (-ax).atan2((ay * ay + az * az).sqrt());
    let yaw_rad = fix
        .true_course_deg
        .map(|course_deg| (course_deg as f64).to_radians())
        .unwrap_or(0.0);

    (roll_rad, pitch_rad, yaw_rad)
}

fn build_navigation_frames(fix: &GpsFix, imu: &ImuReading, elapsed_s: f64) -> NavigationFrames {
    let attitude = EstimatedAttitude::from_snapshot(imu, fix).expect("valid estimated attitude");
    let ned_to_itrf_dcm = itrf_to_ned(fix.latitude_deg.to_radians(), fix.longitude_deg.to_radians()).transpose();

    let earth_itrf = ITRF::<f64>::default();
    let itrf_to_icrf_dcm = itrf_to_icrf(elapsed_s, earth_itrf.angular_velocity());
    let icrf_position_m = itrf_to_icrf_dcm * fix.ecef_m;

    NavigationFrames {
        body_to_ned: attitude.alignment_body_to_ned,
        ned_to_itrf: ned_to_itrf_dcm,
        itrf_to_icrf: itrf_to_icrf_dcm,
        icrf_position_m,
        roll_rad: attitude.roll_rad,
        pitch_rad: attitude.pitch_rad,
        yaw_rad: attitude.yaw_rad,
    }
}

struct GpsState {
    parser: Nmea,
    sentence: String<NMEA_SENTENCE_CAPACITY>,
    reference: Option<GpsReference>,
    last_fix: Option<GpsFix>,
    diagnostics: GpsDiagnostics,
}

impl GpsState {
    fn new() -> Self {
        Self {
            parser: Nmea::default(),
            sentence: String::new(),
            reference: None,
            last_fix: None,
            diagnostics: GpsDiagnostics::new(),
        }
    }

    fn last_fix(&self) -> Option<GpsFix> {
        self.last_fix
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
        }))
    }

    fn poll_uart(&mut self, uart: &mut GpsUart) {
        let mut buffer = [0_u8; 32];

        match uart.read_raw(&mut buffer) {
            Ok(count) => {
                self.diagnostics.saw_rx = self.diagnostics.saw_rx || count > 0;
                self.diagnostics.total_bytes = self.diagnostics.total_bytes.wrapping_add(count as u32);

                for &byte in &buffer[..count] {
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
            }
            Err(nb::Error::WouldBlock) => {}
            Err(nb::Error::Other(_)) => {}
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

    fn read_scaled(&self, i2c: &mut SharedI2cDevice<'a, BUS>) -> Result<(f32, f32, f32, f32, f32, f32), ICMError<E>> {
        match self {
            Self::Addr69(icm) => {
                let raw = icm.get_values_accel_gyro(i2c)?;
                Ok(icm.scale_raw_accel_gyro(raw))
            }
            Self::Addr68(icm) => {
                let raw = icm.get_values_accel_gyro(i2c)?;
                Ok(icm.scale_raw_accel_gyro(raw))
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

    fn read_reg(&self, i2c: &mut SharedI2cDevice<'a, BUS>, reg: u8) -> Result<u8, E> {
        let mut value = [0_u8; 1];
        i2c.write_read(self.address(), &[reg], &mut value)?;
        Ok(value[0])
    }
}

fn init_logs() {
    let channels = rtt_init! {
        up: {
            0: { size: 512, mode: NoBlockSkip, name: "print" }
            1: { size: 512, mode: NoBlockSkip, name: "defmt" }
            2: { size: 1024, mode: NoBlockSkip, name: "telemetry" }
        }
        down: {
            0: { size: 512, mode: NoBlockSkip, name: "commands" }
        }
    };

    set_print_channel(channels.up.0);
}

type GpsUart = UartPeripheral<
    hal::uart::Enabled,
    hal::pac::UART0,
    (
        Pin<hal::gpio::bank0::Gpio12, FunctionUart, hal::gpio::PullDown>,
        Pin<hal::gpio::bank0::Gpio13, FunctionUart, hal::gpio::PullDown>,
    ),
>;

fn log_sensor_snapshot(
    gps: &GpsState,
    hot_start: &mut NavigationHotStart,
    pth: Option<PthReading>,
    imu: Option<ImuReading>,
    elapsed_s: f64,
) {
    match (gps.last_fix(), imu, pth) {
        (Some(fix), Some(imu), pth) => {
            let hot_started = hot_start.maybe_initialize(&fix, &imu, elapsed_s);
            let frames = build_navigation_frames(&fix, &imu, elapsed_s);
            let hot_start_fix = hot_start.initial_fix();
            let hot_start_nav = hot_start.navigator();
            match pth {
                Some(pth) => rprintln!(
                    "gps lat={:.6} lon={:.6} alt={:.1}m sats={:?} hdop={:?} sog={:?} cog={:?} | ecef=[{:.1}, {:.1}, {:.1}]m | ned=[{:.1}, {:.1}, {:.1}]m | icrf=[{:.1}, {:.1}, {:.1}]m | body2ned_rpy=[{:.1}, {:.1}, {:.1}]deg | hot_start={} lat0={:?} lon0={:?} alt0={:?} icrf0={:?} | pth=[{:.2}Pa, {:.2}C, {:.2}%RH] | imu accel=[{:.2}, {:.2}, {:.2}]m/s^2 gyro=[{:.2}, {:.2}, {:.2}]dps",
                    fix.latitude_deg,
                    fix.longitude_deg,
                    fix.altitude_m,
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
                    hot_started || hot_start_nav.is_some(),
                    hot_start_fix.map(|fix| fix.latitude_rad.to_degrees()),
                    hot_start_fix.map(|fix| fix.longitude_rad.to_degrees()),
                    hot_start_fix.map(|fix| fix.altitude_m),
                    hot_start_nav.map(|nav| (nav.position_icrf_m.x(), nav.position_icrf_m.y(), nav.position_icrf_m.z())),
                    pth.pressure_pa,
                    pth.temperature_c,
                    pth.humidity_percent,
                    imu.accel_mps2[0],
                    imu.accel_mps2[1],
                    imu.accel_mps2[2],
                    imu.gyro_dps[0],
                    imu.gyro_dps[1],
                    imu.gyro_dps[2],
                ),
                None => rprintln!(
                    "gps lat={:.6} lon={:.6} alt={:.1}m sats={:?} hdop={:?} sog={:?} cog={:?} | ecef=[{:.1}, {:.1}, {:.1}]m | ned=[{:.1}, {:.1}, {:.1}]m | icrf=[{:.1}, {:.1}, {:.1}]m | body2ned_rpy=[{:.1}, {:.1}, {:.1}]deg | hot_start={} lat0={:?} lon0={:?} alt0={:?} icrf0={:?} | pth=unavailable | imu accel=[{:.2}, {:.2}, {:.2}]m/s^2 gyro=[{:.2}, {:.2}, {:.2}]dps",
                    fix.latitude_deg,
                    fix.longitude_deg,
                    fix.altitude_m,
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
                    hot_started || hot_start_nav.is_some(),
                    hot_start_fix.map(|fix| fix.latitude_rad.to_degrees()),
                    hot_start_fix.map(|fix| fix.longitude_rad.to_degrees()),
                    hot_start_fix.map(|fix| fix.altitude_m),
                    hot_start_nav.map(|nav| (nav.position_icrf_m.x(), nav.position_icrf_m.y(), nav.position_icrf_m.z())),
                    imu.accel_mps2[0],
                    imu.accel_mps2[1],
                    imu.accel_mps2[2],
                    imu.gyro_dps[0],
                    imu.gyro_dps[1],
                    imu.gyro_dps[2],
                ),
            }

            let _ = frames.body_to_ned;
            let _ = frames.ned_to_itrf;
            let _ = frames.itrf_to_icrf;
        }
        (None, Some(imu), pth) => {
            let diagnostics = gps.diagnostics();
            match pth {
                Some(pth) => rprintln!(
                    "gps waiting for fix rx={} bytes={} sentences={} parse_err={} sats={:?} kind={} gga_fix={:?} rmc_status={:?} last='{}' | pth=[{:.2}Pa, {:.2}C, {:.2}%RH] | imu accel=[{:.2}, {:.2}, {:.2}]m/s^2 gyro=[{:.2}, {:.2}, {:.2}]dps",
                    diagnostics.saw_rx,
                    diagnostics.total_bytes,
                    diagnostics.sentence_count,
                    diagnostics.parse_error_count,
                    diagnostics.satellites,
                    diagnostics.last_sentence_kind.as_str(),
                    diagnostics.last_gga_fix_quality,
                    diagnostics.last_rmc_status,
                    diagnostics.last_sentence.as_str(),
                    pth.pressure_pa,
                    pth.temperature_c,
                    pth.humidity_percent,
                    imu.accel_mps2[0],
                    imu.accel_mps2[1],
                    imu.accel_mps2[2],
                    imu.gyro_dps[0],
                    imu.gyro_dps[1],
                    imu.gyro_dps[2],
                ),
                None => rprintln!(
                    "gps waiting for fix rx={} bytes={} sentences={} parse_err={} sats={:?} kind={} gga_fix={:?} rmc_status={:?} last='{}' | pth=unavailable | imu accel=[{:.2}, {:.2}, {:.2}]m/s^2 gyro=[{:.2}, {:.2}, {:.2}]dps",
                    diagnostics.saw_rx,
                    diagnostics.total_bytes,
                    diagnostics.sentence_count,
                    diagnostics.parse_error_count,
                    diagnostics.satellites,
                    diagnostics.last_sentence_kind.as_str(),
                    diagnostics.last_gga_fix_quality,
                    diagnostics.last_rmc_status,
                    diagnostics.last_sentence.as_str(),
                    imu.accel_mps2[0],
                    imu.accel_mps2[1],
                    imu.accel_mps2[2],
                    imu.gyro_dps[0],
                    imu.gyro_dps[1],
                    imu.gyro_dps[2],
                ),
            }
        }
        (Some(fix), None, pth) => {
            match pth {
                Some(pth) => rprintln!(
                    "gps lat={:.6} lon={:.6} alt={:.1}m | ecef=[{:.1}, {:.1}, {:.1}]m | ned=[{:.1}, {:.1}, {:.1}]m | hot_start=false | pth=[{:.2}Pa, {:.2}C, {:.2}%RH] | imu unavailable",
                    fix.latitude_deg,
                    fix.longitude_deg,
                    fix.altitude_m,
                    fix.ecef_m.x(),
                    fix.ecef_m.y(),
                    fix.ecef_m.z(),
                    fix.ned_m.x(),
                    fix.ned_m.y(),
                    fix.ned_m.z(),
                    pth.pressure_pa,
                    pth.temperature_c,
                    pth.humidity_percent,
                ),
                None => rprintln!(
                    "gps lat={:.6} lon={:.6} alt={:.1}m | ecef=[{:.1}, {:.1}, {:.1}]m | ned=[{:.1}, {:.1}, {:.1}]m | hot_start=false | pth=unavailable | imu unavailable",
                    fix.latitude_deg,
                    fix.longitude_deg,
                    fix.altitude_m,
                    fix.ecef_m.x(),
                    fix.ecef_m.y(),
                    fix.ecef_m.z(),
                    fix.ned_m.x(),
                    fix.ned_m.y(),
                    fix.ned_m.z(),
                ),
            }
        }
        (None, None, pth) => {
            let diagnostics = gps.diagnostics();
            match pth {
                Some(pth) => rprintln!(
                    "gps waiting for fix rx={} bytes={} sentences={} parse_err={} sats={:?} kind={} gga_fix={:?} rmc_status={:?} last='{}' | pth=[{:.2}Pa, {:.2}C, {:.2}%RH] | imu unavailable",
                    diagnostics.saw_rx,
                    diagnostics.total_bytes,
                    diagnostics.sentence_count,
                    diagnostics.parse_error_count,
                    diagnostics.satellites,
                    diagnostics.last_sentence_kind.as_str(),
                    diagnostics.last_gga_fix_quality,
                    diagnostics.last_rmc_status,
                    diagnostics.last_sentence.as_str(),
                    pth.pressure_pa,
                    pth.temperature_c,
                    pth.humidity_percent,
                ),
                None => rprintln!(
                    "gps waiting for fix rx={} bytes={} sentences={} parse_err={} sats={:?} kind={} gga_fix={:?} rmc_status={:?} last='{}' | pth=unavailable | imu unavailable",
                    diagnostics.saw_rx,
                    diagnostics.total_bytes,
                    diagnostics.sentence_count,
                    diagnostics.parse_error_count,
                    diagnostics.satellites,
                    diagnostics.last_sentence_kind.as_str(),
                    diagnostics.last_gga_fix_quality,
                    diagnostics.last_rmc_status,
                    diagnostics.last_sentence.as_str(),
                ),
            }
        }
    }
}

#[hal::entry]
fn main() -> ! {
    init_logs();
    rprintln!("ins-test bring-up starting");

    let core = cortex_m::Peripherals::take().unwrap();
    let mut pac = hal::pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let sda_pin = pins.gpio20.reconfigure();
    let scl_pin = pins.gpio21.reconfigure();
    let i2c = hal::I2C::i2c0(
        pac.I2C0,
        sda_pin,
        scl_pin,
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let uart_pins = (
        pins.gpio12.into_function::<FunctionUart>(),
        pins.gpio13.into_function::<FunctionUart>(),
    );
    let mut gps_uart = UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(GPS_BAUD_HZ.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();
    gps_uart.set_fifos(true);

    let i2c_bus = RefCell::new(i2c);

    let mut ms8607 = {
        let sensor_bus = SharedI2cDevice::new(&i2c_bus);
        let mut sensor = MS8607::new(sensor_bus);
        match sensor.begin(&mut delay) {
            Ok(()) => rprintln!("ms8607 ready on gp20/gp21"),
            Err(_) => rprintln!("ms8607 init failed"),
        }
        sensor
    };

    let icm = {
        let mut sensor_bus = SharedI2cDevice::new(&i2c_bus);
        match IcmDevice::init(&mut delay, &mut sensor_bus) {
            Ok(device) => {
                if device.wake_sensors(&mut sensor_bus).is_err() {
                    rprintln!("icm20948 wake sequence failed");
                }

                let who_am_i = device.read_reg(&mut sensor_bus, ICM_WHO_AM_I_REG).ok();
                let pwr_mgmt_1 = device.read_reg(&mut sensor_bus, ICM_PWR_MGMT_1_REG).ok();
                let pwr_mgmt_2 = device.read_reg(&mut sensor_bus, ICM_PWR_MGMT_2_REG).ok();

                rprintln!("icm20948 ready at 0x{:02x}", device.address());
                rprintln!(
                    "icm20948 whoami={:?} pwr_mgmt_1={:?} pwr_mgmt_2={:?}",
                    who_am_i,
                    pwr_mgmt_1,
                    pwr_mgmt_2,
                );
                Some(device)
            }
            Err(_) => {
                rprintln!("icm20948 init failed on 0x69 and 0x68");
                None
            }
        }
    };

    rprintln!("gps uart ready on gp12/gp13 at {} baud", GPS_BAUD_HZ);

    let mut gps = GpsState::new();
    let mut hot_start = NavigationHotStart::new();
    let mut poll_counter = 0_u32;
    let mut uptime_ms = 0_u64;

    loop {
        gps.poll_uart(&mut gps_uart);

        poll_counter = poll_counter.wrapping_add(1);
        if poll_counter >= SENSOR_POLL_DIVIDER {
            poll_counter = 0;

            let pth = ms8607.get_measurements(&mut delay).ok().map(PthReading::from);
            let imu = icm.as_ref().and_then(|device| {
                let mut sensor_bus = SharedI2cDevice::new(&i2c_bus);
                device.read_scaled(&mut sensor_bus).ok().map(ImuReading::from)
            });

            log_sensor_snapshot(&gps, &mut hot_start, pth, imu, uptime_ms as f64 * 1.0e-3);
        }

        delay.delay_ms(20_u32);
        uptime_ms = uptime_ms.wrapping_add(20);
    }
}

#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    hal::binary_info::rp_cargo_bin_name!(),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"INS test GPS + ECEF/NED + PTH"),
    hal::binary_info::rp_cargo_homepage_url!(),
    hal::binary_info::rp_program_build_attribute!(),
];
