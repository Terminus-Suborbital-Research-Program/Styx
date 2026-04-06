use aether::{
    attitude::{DirectionCosineMatrix, Quaternion},
    math::Vector,
    reference_frame::{Body, ICRF, ITRF, NED},
};
use bmi323::{
    AccelConfig, AccelerometerRange, AverageNum, Bandwidth, Bmi323, Error as BmiError,
    GyroConfig, GyroscopePowerMode, GyroscopeRange, OutputDataRate, Sensor3DDataScaled,
};
use bmm350::{
    AverageNum as BmmAverageNum, AxisEnableDisable, Bmm350, DataRate, Error as BmmError,
    MagConfig, PerformanceMode, PowerMode, Sensor3DData,
};
use embedded_hal::i2c::I2c;
use linux_embedded_hal::{Delay, I2cdev};
use nmea::Nmea;
use rs_ws281x::{ChannelBuilder, Controller, ControllerBuilder, StripType};
use std::{
    env,
    io::{self, Read},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const LED_COUNT: i32 = 4;
const BMI323_I2C_ADDR: u8 = 0x69;
const BMM350_I2C_ADDR: u8 = 0x14;
const GPS_I2C_ADDR: u8 = 0x42;
const GPS_I2C_BYTES_AVAILABLE_REGISTER: u8 = 0xFD;
const GPS_I2C_STREAM_REGISTER: u8 = 0xFF;
const GPS_I2C_MAX_READ: usize = 32;
const EARTH_ROTATION_RATE_RAD_PER_S: f64 = 7.292_115_0e-5;
const WGS84_A_M: f64 = 6_378_137.0;
const WGS84_E2: f64 = 6.694_379_990_14e-3;

fn bmi_err<E: core::fmt::Debug>(err: BmiError<E>) -> std::io::Error {
    std::io::Error::other(format!("bmi323 error: {:?}", err))
}

fn bmm_err<E: core::fmt::Debug>(err: BmmError<E>) -> std::io::Error {
    std::io::Error::other(format!("bmm350 error: {:?}", err))
}

#[derive(Clone, Debug)]
struct GpsData {
    geodetic_deg_m: Vector<f64, 3>,
    satellites: Option<u32>,
    hdop: Option<f32>,
    speed_knots: Option<f32>,
    true_course_deg: Option<f32>,
}

#[derive(Clone, Debug)]
struct State {
    unix_timestamp_s: u64,
    geodetic_deg_m: Option<Vector<f64, 3>>,
    ecef_m: Option<[f64; 3]>,
    body_to_ned: Quaternion<f64, Body<f64>, NED<f64>>,
    body_to_icrf: Option<Quaternion<f64, Body<f64>, ICRF<f64>>>,
    satellites: Option<u32>,
    hdop: Option<f32>,
    yaw_deg: f32,
    magnetic_heading_deg: f32,
    relative_magnetic_heading_deg: f32,
    roll_deg: f32,
    pitch_deg: f32,
    accel_mps2: Sensor3DDataScaled,
    gyro_dps: Sensor3DDataScaled,
    speed_knots: Option<f32>,
    true_course_deg: Option<f32>,
    north_body: [f64; 3],
    east_body: [f64; 3],
    down_body: [f64; 3],
}

fn wrap_angle_deg(angle_deg: f32) -> f32 {
    let wrapped = angle_deg % 360.0;
    if wrapped < 0.0 {
        wrapped + 360.0
    } else {
        wrapped
    }
}

fn unix_timestamp_s_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn extract_gps_fix(nmea: &Nmea) -> Option<GpsData> {
    let latitude = nmea.latitude()?;
    let longitude = nmea.longitude()?;
    let altitude_m = nmea.altitude().unwrap_or_default() as f64;

    Some(GpsData {
        geodetic_deg_m: Vector::new([latitude, longitude, altitude_m]),
        satellites: nmea.fix_satellites(),
        hdop: nmea.hdop(),
        speed_knots: nmea.speed_over_ground,
        true_course_deg: nmea.true_course,
    })
}

fn magnetic_heading_deg(sample: Sensor3DData) -> f32 {
    ((sample.y as f32).atan2(sample.x as f32).to_degrees()).rem_euclid(360.0)
}

fn relative_heading_deg(current_deg: f32, reference_deg: f32) -> f32 {
    (current_deg - reference_deg).rem_euclid(360.0)
}

fn tilt_roll_deg(accel: Sensor3DDataScaled) -> f32 {
    accel.y.atan2(accel.z).to_degrees()
}

fn tilt_pitch_deg(accel: Sensor3DDataScaled) -> f32 {
    (-accel.x)
        .atan2((accel.y * accel.y + accel.z * accel.z).sqrt())
        .to_degrees()
}

fn norm3(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn normalize3(v: [f64; 3]) -> [f64; 3] {
    let n = norm3(v);
    if n <= f64::EPSILON {
        [0.0, 0.0, 0.0]
    } else {
        [v[0] / n, v[1] / n, v[2] / n]
    }
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale3(v: [f64; 3], scalar: f64) -> [f64; 3] {
    [v[0] * scalar, v[1] * scalar, v[2] * scalar]
}

fn hsv_to_rgb(hue_deg: f32, saturation: f32, value: f32) -> [u8; 4] {
    let h = hue_deg.rem_euclid(360.0);
    let c = value * saturation;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = value - c;

    let (r1, g1, b1) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    [
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
        0,
    ]
}

fn render_rgb_color_wheel(controller: &mut Controller, phase_deg: f32) -> Result<(), Box<dyn std::error::Error>> {
    let leds = controller.leds_mut(0);
    for i in 0..LED_COUNT as usize {
        let hue = phase_deg + (i as f32) * (360.0 / LED_COUNT as f32);
        leds[i] = hsv_to_rgb(hue, 1.0, 1.0);
    }
    controller.render()?;
    Ok(())
}

fn render_orange_spin(
    controller: &mut Controller,
    phase_deg: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    let led_pos = (phase_deg % 360.0) / 90.0;
    let current_led = (led_pos.floor() as usize) % 4;
    let next_led = (current_led + 1) % 4;
    let frac = led_pos.fract();

    let leds = controller.leds_mut(0);
    for i in 0..LED_COUNT as usize {
        let color = if i == current_led {
            let t = 1.0 - frac;
            let r = (255.0 * t) as u8;
            let g = (128.0 * (1.0 - t) + 64.0 * t) as u8;
            [r, g, 0, 0]
        } else if i == next_led {
            let t = frac;
            let r = (255.0 * t) as u8;
            let g = (128.0 * (1.0 - t) + 64.0 * t) as u8;
            [r, g, 0, 0]
        } else {
            [0, 0, 0, 0]
        };
        leds[i] = color;
    }

    controller.render()?;
    Ok(())
}

fn format_state_line(state: &State) -> String {
    let gps_line = if let Some(geodetic) = state.geodetic_deg_m {
        format!(
            "GPS:    lat={:.6}, long={:.6}, alt={:.2}m, sats={:?}, hdop={:?}, sog={:?}kt, course={:?}°",
            geodetic[0],
            geodetic[1],
            geodetic[2],
            state.satellites,
            state.hdop,
            state.speed_knots,
            state.true_course_deg
        )
    } else {
        format!(
            "GPS:    waiting, sats={:?}, hdop={:?}, sog={:?}kt, course={:?}°",
            state.satellites,
            state.hdop,
            state.speed_knots,
            state.true_course_deg
        )
    };

    let ecef_line = if let Some(ecef) = state.ecef_m {
        format!("ECEF:   x={:.2}, y={:.2}, z={:.2}", ecef[0], ecef[1], ecef[2])
    } else {
        "ECEF:   waiting".to_string()
    };

    format!(
        "=====================
TIME:   ts={}
YPR:    yaw={:+8.3}   roll={:+8.3}   pitch={:+8.3}
MAG:    hdg={:6.2}    rel={:6.2}
ATT:    q_ned={}   q_icrf={:?}
NED:    N={:>8.2?}   E={:>8.2?}   D={:>8.2?}
{}
{}
=====================",
        state.unix_timestamp_s,
        state.yaw_deg,
        state.roll_deg,
        state.pitch_deg,
        state.magnetic_heading_deg,
        state.relative_magnetic_heading_deg,
        state.body_to_ned,
        state.body_to_icrf,
        state.north_body,
        state.east_body,
        state.down_body,
        gps_line,
        ecef_line,
    )
}

fn geodetic_deg_to_ecef_m(geodetic_deg_m: Vector<f64, 3>) -> [f64; 3] {
    let latitude = geodetic_deg_m[0].to_radians();
    let longitude = geodetic_deg_m[1].to_radians();
    let altitude = geodetic_deg_m[2];

    let sin_lat = latitude.sin();
    let cos_lat = latitude.cos();
    let sin_lon = longitude.sin();
    let cos_lon = longitude.cos();

    let n = WGS84_A_M / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();

    [
        (n + altitude) * cos_lat * cos_lon,
        (n + altitude) * cos_lat * sin_lon,
        (n * (1.0 - WGS84_E2) + altitude) * sin_lat,
    ]
}

fn itrf_to_ned_dcm(latitude_rad: f64, longitude_rad: f64) -> DirectionCosineMatrix<f64, ITRF<f64>, NED<f64>> {
    DirectionCosineMatrix::new(
        -latitude_rad.sin() * longitude_rad.cos(),
        -latitude_rad.sin() * longitude_rad.sin(),
        latitude_rad.cos(),
        -longitude_rad.sin(),
        longitude_rad.cos(),
        0.0,
        -latitude_rad.cos() * longitude_rad.cos(),
        -latitude_rad.cos() * longitude_rad.sin(),
        -latitude_rad.sin(),
    )
}

fn itrf_to_icrf_dcm(time_s: f64) -> DirectionCosineMatrix<f64, ITRF<f64>, ICRF<f64>> {
    let theta = EARTH_ROTATION_RATE_RAD_PER_S * time_s;
    DirectionCosineMatrix::new(
        theta.cos(),
        -theta.sin(),
        0.0,
        theta.sin(),
        theta.cos(),
        0.0,
        0.0,
        0.0,
        1.0,
    )
}

fn body_to_ned_from_sensors(
    accel: Sensor3DDataScaled,
    mag: Sensor3DData,
) -> (DirectionCosineMatrix<f64, Body<f64>, NED<f64>>, [f64; 3], [f64; 3], [f64; 3]) {
    let down_body = normalize3([
        accel.x as f64,
        accel.y as f64,
        accel.z as f64,
    ]);

    let mag_body = [mag.x as f64, mag.y as f64, mag.z as f64];
    let mag_along_down = scale3(down_body, dot3(mag_body, down_body));
    let mag_horizontal = sub3(mag_body, mag_along_down);

    let mut north_body = normalize3(mag_horizontal);
    if norm3(north_body) <= f64::EPSILON {
        north_body = [1.0, 0.0, 0.0];
    }

    let mut east_body = normalize3(cross3(down_body, north_body));
    if norm3(east_body) <= f64::EPSILON {
        east_body = [0.0, 1.0, 0.0];
    }

    north_body = normalize3(cross3(east_body, down_body));

    (
        DirectionCosineMatrix::new(
            north_body[0],
            north_body[1],
            north_body[2],
            east_body[0],
            east_body[1],
            east_body[2],
            down_body[0],
            down_body[1],
            down_body[2],
        ),
        north_body,
        east_body,
        down_body,
    )
}

fn parse_i2c_addr(value: &str) -> Option<u8> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u8::from_str_radix(hex, 16).ok()
    } else {
        trimmed.parse::<u8>().ok()
    }
}

fn gps_bytes_available(i2c: &mut I2cdev, gps_addr: u8) -> std::io::Result<usize> {
    let mut available = [0_u8; 2];
    i2c.write_read(gps_addr, &[GPS_I2C_BYTES_AVAILABLE_REGISTER], &mut available)
        .map_err(|err| std::io::Error::other(format!("gps i2c count read failed: {err}")))?;

    Ok((((available[0] as usize) << 8) | available[1] as usize) & 0x7FFF)
}

fn gps_read_chunk(i2c: &mut I2cdev, gps_addr: u8, buffer: &mut [u8]) -> std::io::Result<usize> {
    i2c.write_read(gps_addr, &[GPS_I2C_STREAM_REGISTER], buffer)
        .map_err(|err| std::io::Error::other(format!("gps i2c data read failed: {err}")))?;
    Ok(buffer.len())
}

fn start_gps_reader() -> Arc<Mutex<Option<GpsData>>> {
    let gps_bus = env::var("MUNIN_GPS_I2C_BUS").unwrap_or_else(|_| "/dev/i2c-1".to_string());
    let gps_addr = env::var("MUNIN_GPS_I2C_ADDR")
        .ok()
        .and_then(|value| parse_i2c_addr(&value))
        .unwrap_or(GPS_I2C_ADDR);

    let shared_fix = Arc::new(Mutex::new(None));
    let gps_state = Arc::clone(&shared_fix);

    thread::Builder::new()
        .name("gps-reader".into())
        .spawn(move || {
            let mut i2c = match I2cdev::new(&gps_bus) {
                Ok(i2c) => i2c,
                Err(err) => {
                    eprintln!(
                        "GPS disabled: failed to open I2C bus {} for address 0x{gps_addr:02X}: {}",
                        gps_bus, err
                    );
                    return;
                }
            };

            eprintln!(
                "GPS reader listening on {} at I2C address 0x{gps_addr:02X}",
                gps_bus
            );

            let mut parser = Nmea::default();
            let mut sentence = String::new();
            let mut chunk = [0_u8; GPS_I2C_MAX_READ];

            loop {
                match gps_bytes_available(&mut i2c, gps_addr) {
                    Ok(0) => {
                        thread::sleep(Duration::from_millis(100));
                    }
                    Ok(available) => {
                        let to_read = available.min(chunk.len());

                        match gps_read_chunk(&mut i2c, gps_addr, &mut chunk[..to_read]) {
                            Ok(read) => {
                                for &byte in &chunk[..read] {
                                    match byte {
                                        b'\r' => {}
                                        b'\n' => {
                                            let line = sentence.trim();
                                            if !line.is_empty() && parser.parse(line).is_ok() {
                                                if let Some(fix) = extract_gps_fix(&parser) {
                                                    if let Ok(mut latest) = gps_state.lock() {
                                                        *latest = Some(fix);
                                                    }
                                                }
                                            }
                                            sentence.clear();
                                        }
                                        b'$' => {
                                            sentence.clear();
                                            sentence.push('$');
                                        }
                                        byte if sentence.is_empty() => {}
                                        byte if byte.is_ascii() => {
                                            sentence.push(byte as char);
                                        }
                                        _ => {
                                            sentence.clear();
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                eprintln!(
                                    "GPS read error on {} at 0x{gps_addr:02X}: {}",
                                    gps_bus, err
                                );
                                thread::sleep(Duration::from_millis(500));
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!(
                            "GPS status error on {} at 0x{gps_addr:02X}: {}",
                            gps_bus, err
                        );
                        thread::sleep(Duration::from_millis(500));
                    }
                }
            }
        })
        .expect("failed to spawn GPS reader thread");

    shared_fix
}



fn gps_fix_is_locked(gps_fix: &Arc<Mutex<Option<GpsData>>>) -> bool {
    gps_fix.lock().ok().and_then(|latest| latest.clone()).is_some()
}

fn wait_for_gps_lock(
    controller: &mut Controller,
    gps_fix: &Arc<Mutex<Option<GpsData>>>,
    skip_gps_lock: &Arc<Mutex<bool>>,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!("Waiting for GPS lock. Press 's' + Enter to skip this step for indoor testing.");

    let wait_start = Instant::now();
    loop {
        if gps_fix_is_locked(gps_fix) {
            println!("GPS lock acquired.");
            return Ok(true);
        }

        if *skip_gps_lock.lock().unwrap() {
            println!("GPS lock skipped.");
            return Ok(false);
        }

        let elapsed = wait_start.elapsed().as_secs_f32();
        let phase_deg = elapsed * 240.0 + 30.0;
        render_orange_spin(controller, phase_deg)?;

        thread::sleep(Duration::from_millis(20));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gps_fix = start_gps_reader();

    let mut controller: Controller = ControllerBuilder::new()
        .freq(800_000)
        .dma(10)
        .channel(
            0,
            ChannelBuilder::new()
                .pin(18)
                .count(LED_COUNT)
                .strip_type(StripType::Ws2812)
                .brightness(180)
                .build(),
        )
        .build()?;

    let imu_i2c = I2cdev::new("/dev/i2c-1")?;
    let mag_i2c = I2cdev::new("/dev/i2c-1")?;
    let mut imu: Bmi323<_, _> = Bmi323::new_with_i2c(imu_i2c, BMI323_I2C_ADDR, Delay);
    let mut mag: Bmm350<_, _> = Bmm350::new_with_i2c(mag_i2c, BMM350_I2C_ADDR, Delay);

    println!("=== BMI323 + BMM350 Mount Scaffold ===");

    println!("Running RGB wheel startup pattern while sensors initialize...");
    let wheel_start = Instant::now();
    while wheel_start.elapsed() < Duration::from_secs(3) {
        let elapsed = wheel_start.elapsed().as_secs_f32();
        let phase_deg = elapsed * 240.0;
        render_rgb_color_wheel(&mut controller, phase_deg)?;

        thread::sleep(Duration::from_millis(20));
    }

    println!("Initializing BMI323 sensor...");
    imu.init().map_err(bmi_err)?;
    imu.set_accel_config(
        AccelConfig::builder()
            .odr(OutputDataRate::Odr100hz)
            .range(AccelerometerRange::G8)
            .bw(Bandwidth::OdrQuarter)
            .avg_num(AverageNum::Avg4)
            .build(),
    )
    .map_err(bmi_err)?;
    imu.set_gyro_config(
        GyroConfig::builder()
            .odr(OutputDataRate::Odr100hz)
            .range(GyroscopeRange::DPS2000)
            .avg_num(AverageNum::Avg4)
            .mode(GyroscopePowerMode::Normal)
            .build(),
    )
    .map_err(bmi_err)?;
    println!("BMI323 configured.");

    println!("Initializing BMM350 sensor...");
    thread::sleep(Duration::from_millis(100));

    let mut mag_init_ok = false;
    let mut last_mag_err: Option<String> = None;

    for attempt in 0..8 {
        let _ = mag.set_power_mode(PowerMode::Suspend);
        thread::sleep(Duration::from_millis(50));

        if let Err(err) = mag.magnetic_reset() {
            eprintln!(
                "BMM350 magnetic reset attempt {} failed: {:?}",
                attempt + 1,
                err
            );
            last_mag_err = Some(format!("reset attempt {}: {:?}", attempt + 1, err));
        }

        thread::sleep(Duration::from_millis(500 + (attempt as u64) * 250));

        match mag.init() {
            Ok(()) => {
                mag_init_ok = true;
                break;
            }
            Err(err) => {
                eprintln!(
                    "BMM350 init attempt {} failed: {:?}",
                    attempt + 1,
                    err
                );
                last_mag_err = Some(format!("init attempt {}: {:?}", attempt + 1, err));
                thread::sleep(Duration::from_millis(250));
            }
        }
    }

    if !mag_init_ok {
        return Err(Box::new(std::io::Error::other(format!(
            "failed to initialize BMM350 after repeated suspend/reset attempts: {}",
            last_mag_err.unwrap_or_else(|| "unknown error".to_string())
        ))));
    }

    mag.enable_axes(
        AxisEnableDisable::Enable,
        AxisEnableDisable::Enable,
        AxisEnableDisable::Enable,
    )
    .map_err(bmm_err)?;
    mag.set_odr_performance(DataRate::ODR25Hz, BmmAverageNum::Avg4)
        .map_err(bmm_err)?;
    mag.set_mag_config(
        MagConfig::builder()
            .odr(DataRate::ODR25Hz)
            .performance(PerformanceMode::Regular)
            .mode(PowerMode::Normal)
            .build(),
    )
    .map_err(bmm_err)?;
    mag.set_power_mode(PowerMode::Normal).map_err(bmm_err)?;
    println!("BMM350 configured.");

    let shutdown = Arc::new(Mutex::new(false));
    let shutdown_clone = Arc::clone(&shutdown);
    let gps_skip_requested = Arc::new(Mutex::new(false));
    let gps_skip_clone = Arc::clone(&gps_skip_requested);

    thread::spawn(move || {
        let mut buffer = [0u8; 1];
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        while handle.read_exact(&mut buffer).is_ok() {
            match buffer[0] {
                b'q' | b'Q' => {
                    let mut s = shutdown_clone.lock().unwrap();
                    *s = true;
                    println!("\nShutdown requested - cleaning up sensors...");
                    break;
                }
                b's' | b'S' => {
                    let mut skip = gps_skip_clone.lock().unwrap();
                    *skip = true;
                    println!("\nGPS lock skip requested.");
                }
                _ => {}
            }
        }
    });

    println!("Calibrating gyro bias and magnetic reference — keep the mount still for 2 seconds...");
    println!("Watch the green LED spin smoothly around the ring.");

    let cal_start = Instant::now();
    let mut gyro_bias_z = 0.0_f32;
    let mut magnetic_reference_deg = 0.0_f32;
    let mut sample_count: u32 = 0;

    while cal_start.elapsed() < Duration::from_secs(2) {
        let gyro = imu.read_gyro_data_scaled().map_err(bmi_err)?;
        let mag_data = mag.read_mag_data().map_err(bmm_err)?;
        gyro_bias_z += gyro.z;
        magnetic_reference_deg += magnetic_heading_deg(mag_data);
        sample_count += 1;

        let elapsed = cal_start.elapsed().as_secs_f32();
        let progress = elapsed / 2.0;
        let cal_angle = progress * 720.0;
        let led_pos = (cal_angle % 360.0) / 90.0;
        let current_led = (led_pos.floor() as usize) % 4;
        let next_led = (current_led + 1) % 4;
        let frac = led_pos.fract();

        let leds = controller.leds_mut(0);
        for i in 0..LED_COUNT as usize {
            let color = if i == current_led {
                let t = 1.0 - frac;
                let g = (t * 255.0) as u8;
                [0, g, 0, 0]
            } else if i == next_led {
                let t = frac;
                let g = (t * 255.0) as u8;
                [0, g, 0, 0]
            } else {
                [0, 0, 0, 0]
            };
            leds[i] = color;
        }
        controller.render()?;

        thread::sleep(Duration::from_millis(20));
    }

    if sample_count > 0 {
        gyro_bias_z /= sample_count as f32;
        magnetic_reference_deg /= sample_count as f32;
    }

    println!("✅ Calibration complete! Gyro Z bias: {:.2} dps", gyro_bias_z);
    println!(
        "✅ Magnetic reference complete! BMM350 north reference: {:.2}°",
        magnetic_reference_deg
    );

    println!("Running GPS lock wait with orange spin...");
    let _ = wait_for_gps_lock(&mut controller, &gps_fix, &gps_skip_requested)?;

    println!("Scaffold assumes BMI323 at 0x69 and BMM350 at 0x14.");
    println!("LED ring currently follows BMM350 relative heading for bench testing.");
    println!(
        "GPS expects u-blox DDC/NMEA on $MUNIN_GPS_I2C_BUS or /dev/i2c-1 at $MUNIN_GPS_I2C_ADDR or 0x42."
    );
    println!("Type 'q' + Enter to safely shutdown.");

    let mut last_print = Instant::now();
    let mut last_time = Instant::now();
    let mut yaw_deg = 0.0_f32;

    loop {
        if *shutdown.lock().unwrap() {
            let _ = controller.leds_mut(0).iter_mut().map(|led| *led = [0, 0, 0, 0]).count();
            let _ = controller.render();
            let _ = mag.set_power_mode(PowerMode::Suspend);
            thread::sleep(Duration::from_millis(50));
            let _ = mag.magnetic_reset();
            println!("Sensors disabled. Goodbye.");
            break;
        }

        let accel = imu.read_accel_data_scaled().map_err(bmi_err)?;
        let gyro = imu.read_gyro_data_scaled().map_err(bmi_err)?;
        let mag_data = mag.read_mag_data().map_err(bmm_err)?;

        let now = Instant::now();
        let dt = (now - last_time).as_secs_f32();
        last_time = now;

        let corrected_gz = gyro.z - gyro_bias_z;
        if corrected_gz.abs() > 0.5 {
            yaw_deg = wrap_angle_deg(yaw_deg + corrected_gz * dt);
        }

        let magnetic_heading = magnetic_heading_deg(mag_data);
        let relative_magnetic_heading =
            relative_heading_deg(magnetic_heading, magnetic_reference_deg);
        let roll_deg = tilt_roll_deg(accel);
        let pitch_deg = tilt_pitch_deg(accel);

        let led_pos = yaw_deg / 90.0;
        let current_led = (led_pos.floor() as usize) % 4;
        let next_led = (current_led + 1) % 4;
        let frac = led_pos.fract();

        let leds = controller.leds_mut(0);
        for i in 0..LED_COUNT as usize {
            let color: [u8; 4] = if i == current_led {
                let t = 1.0 - frac;
                let r = (t * 255.0) as u8;
                let b = ((1.0 - t) * 255.0) as u8;
                [r, 0, b, 0]
            } else if i == next_led {
                let t = frac;
                let r = (t * 255.0) as u8;
                let b = ((1.0 - t) * 255.0) as u8;
                [r, 0, b, 0]
            } else {
                [0, 0, 255, 0]
            };
            leds[i] = color;
        }

        controller.render()?;

        if last_print.elapsed() > Duration::from_millis(300) {
            let gps_data = gps_fix.lock().ok().and_then(|latest| latest.clone());
            let unix_timestamp_s = unix_timestamp_s_now();

            let (body_to_ned_dcm, north_body, east_body, down_body) =
                body_to_ned_from_sensors(accel, mag_data);

            let body_to_ned = Quaternion::try_from(&body_to_ned_dcm)
                .unwrap_or_else(|_| Quaternion::<f64, Body<f64>, NED<f64>>::identity())
                .normalized();

            let geodetic_deg_m = gps_data.as_ref().map(|gps| gps.geodetic_deg_m);
            let ecef_m = geodetic_deg_m.map(geodetic_deg_to_ecef_m);

            let body_to_icrf = geodetic_deg_m.and_then(|geodetic| {
                let latitude_rad = geodetic[0].to_radians();
                let longitude_rad = geodetic[1].to_radians();

                let ned_to_itrf = itrf_to_ned_dcm(latitude_rad, longitude_rad).transpose();
                let body_to_itrf = ned_to_itrf * body_to_ned_dcm;
                let body_to_icrf_dcm = itrf_to_icrf_dcm(unix_timestamp_s as f64) * body_to_itrf;

                Quaternion::try_from(&body_to_icrf_dcm).ok().map(|q| q.normalized())
            });

            let state = State {
                unix_timestamp_s,
                geodetic_deg_m,
                ecef_m,
                body_to_ned,
                body_to_icrf,
                satellites: gps_data.as_ref().and_then(|gps| gps.satellites),
                hdop: gps_data.as_ref().and_then(|gps| gps.hdop),
                yaw_deg,
                magnetic_heading_deg: magnetic_heading,
                relative_magnetic_heading_deg: relative_magnetic_heading,
                roll_deg,
                pitch_deg,
                accel_mps2: accel,
                gyro_dps: gyro,
                speed_knots: gps_data.as_ref().and_then(|gps| gps.speed_knots),
                true_course_deg: gps_data.as_ref().and_then(|gps| gps.true_course_deg),
                north_body,
                east_body,
                down_body,
            };

            println!("{}", format_state_line(&state));

            last_print = Instant::now();
        }

        thread::sleep(Duration::from_millis(50));
    }

    Ok(())
}
