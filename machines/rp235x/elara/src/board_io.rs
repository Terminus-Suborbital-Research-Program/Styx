#![cfg(all(target_arch = "arm", target_os = "none"))]

use core::fmt::Write as _;

use actuators::bldc::{
    BldcEsc, ESC_MAX_PULSE_WIDTH_US, ESC_MIN_PULSE_WIDTH_US, ESC_NEUTRAL_PULSE_WIDTH_US,
};
use actuators::servo::NoPowerGate;
use embedded_hal::digital::OutputPin;
use fugit::RateExtU32;
use heapless::String;
use bmi323::{
    AccelConfig, AccelerometerPowerMode, AccelerometerRange, AverageNum, Bandwidth, Bmi323,
    GyroConfig, GyroscopePowerMode, GyroscopeRange, OutputDataRate,
};
use bmm350::{
    AverageNum as BmmAverageNum, AxisEnableDisable, Bmm350, DataRate, MagConfig, PerformanceMode,
    PowerMode,
};
use pulsar::arch::Arch;
use pulsar::sync::ArbiterDevice;
use rp235x_hal as hal;
use rp235x_hal::gpio::{FunctionI2C, FunctionSio, FunctionUart, Pin, PullDown, SioOutput};
use rp235x_hal::uart::{DataBits, Enabled, StopBits, UartConfig, UartPeripheral};
use rp235x_hal::Clock;
use rtt_target::{rprintln, UpChannel};

use crate::device_constants::{
    BMM350_I2C_ADDR, BMI323_I2C_ADDR, BldcX, BldcY, BldcZ, BmmDevice, BmiDevice,
    ComputeRxPin, ComputeTxPin, ESC_PWM_DIV_INT, GpsRxPin, GpsTxPin, I2C_BAUD_HZ,
    SatelliteI2cArbiter, SensorDelay, TestMaxEsc, TestMinEsc,
};
use elara::{
    ComputeMessage, GpsFix, TelemetryFrame, TelemetrySnapshot, COMPUTE_LINK_FLAG,
    COMPUTE_UART_BAUD, GPS_UART_BAUD, GPS_VALID_FLAG, IMU_VALID_FLAG, MAG_VALID_FLAG,
};

type ComputeUart = UartPeripheral<
    Enabled,
    hal::pac::UART0,
    (
        Pin<ComputeTxPin, FunctionUart, PullDown>,
        Pin<ComputeRxPin, FunctionUart, PullDown>,
    ),
>;

type GpsUart = UartPeripheral<
    Enabled,
    hal::pac::UART1,
    (
        Pin<GpsTxPin, FunctionUart, PullDown>,
        Pin<GpsRxPin, FunctionUart, PullDown>,
    ),
>;

pub(super) struct Board {
    led: Pin<hal::gpio::bank0::Gpio25, FunctionSio<SioOutput>, PullDown>,
    _test_max_esc: TestMaxEsc,
    _test_min_esc: TestMinEsc,
    _shared_i2c: &'static SatelliteI2cArbiter,
    bmi323: Option<BmiDevice>,
    bmm350: Option<BmmDevice>,
    delay: SensorDelay,
    gps_uart: GpsUart,
    compute_uart: ComputeUart,
    esc_x: BldcX,
    esc_y: BldcY,
    esc_z: BldcZ,
    telemetry_seq: u32,
    compute_rx_count: u32,
    compute_line: String<128>,
    esc_pulse_widths_us: [u16; 3],
    esc_pwm_top: u16,
    sample_debug_budget: u8,
    system_clock_hz: u32,
}

const GNC_ESC_STOP_HOLD_MS: u32 = 2_000;
const GNC_ESC_NEUTRAL_RAMP_DURATION_MS: u32 = 1_000;
const GNC_ESC_TRIM_RAMP_DURATION_MS: u32 = 1_000;
const GNC_ESC_STARTUP_SWEEP_TARGET_US: u16 = ESC_NEUTRAL_PULSE_WIDTH_US as u16;
const TEST_MAX_PWM_PULSE_WIDTH_US: u16 = 2_000;
const TEST_MIN_PWM_PULSE_WIDTH_US: u16 = 1_000;

enum GncEscStartupPhase {
    Stop,
    PulseSweep { pulse_width_us: u16 },
    Active { trim_scale_ms: u32 },
}

impl Board {
    fn esc_pwm_top(system_clock_hz: u32) -> u16 {
        (((system_clock_hz as u64) / ESC_PWM_DIV_INT as u64) / 50 - 1) as u16
    }

    pub(super) fn new(pac: hal::pac::Peripherals) -> Result<Self, &'static str> {
        rprintln!("boot: board init begin");
        let hal::pac::Peripherals {
            WATCHDOG,
            XOSC,
            CLOCKS,
            PLL_SYS,
            PLL_USB,
            mut RESETS,
            SIO,
            IO_BANK0,
            PADS_BANK0,
            I2C0,
            TIMER0,
            PWM,
            UART0,
            UART1,
            ..
        } = pac;

        let mut watchdog = hal::Watchdog::new(WATCHDOG);

        let clocks = hal::clocks::init_clocks_and_plls(
            crate::device_constants::XTAL_FREQ_HZ,
            XOSC,
            CLOCKS,
            PLL_SYS,
            PLL_USB,
            &mut RESETS,
            &mut watchdog,
        )
        .map_err(|_| "clock init failed")?;
        rprintln!("boot: clocks initialized");

        let system_clock_hz = clocks.system_clock.freq().to_Hz();
        let esc_pwm_top = Self::esc_pwm_top(system_clock_hz);
        let mut delay = hal::Timer::new_timer0(TIMER0, &mut RESETS, &clocks);

        let sio = hal::Sio::new(SIO);
        let pins = hal::gpio::Pins::new(IO_BANK0, PADS_BANK0, sio.gpio_bank0, &mut RESETS);

        let led = pins.gpio25.into_push_pull_output();

        let sda_pin: Pin<_, FunctionI2C, _> = pins.gpio4.reconfigure();
        let scl_pin: Pin<_, FunctionI2C, _> = pins.gpio5.reconfigure();
        let i2c = hal::I2C::i2c0(
            I2C0,
            sda_pin,
            scl_pin,
            I2C_BAUD_HZ.Hz(),
            &mut RESETS,
            &clocks.system_clock,
        );
        let shared_i2c = cortex_m::singleton!(: SatelliteI2cArbiter = SatelliteI2cArbiter::new(i2c))
            .ok_or("shared i2c already initialized")?;
        rprintln!("boot: i2c initialized");

        let bmi323 = match Self::init_bmi323(shared_i2c, delay) {
            Some(sensor) => Some(sensor),
            None => {
                rprintln!("boot: bmi323 unavailable; continuing without imu");
                None
            }
        };

        let bmm350 = match Self::init_bmm350(shared_i2c, delay) {
            Some(sensor) => Some(sensor),
            None => {
                rprintln!("boot: bmm350 unavailable; continuing without magnetometer");
                None
            }
        };

        let compute_uart = UartPeripheral::new(
            UART0,
            (pins.gpio0.into_function(), pins.gpio1.into_function()),
            &mut RESETS,
        )
        .enable(
            UartConfig::new(COMPUTE_UART_BAUD.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .map_err(|_| "compute uart init failed")?;
        rprintln!("boot: compute uart initialized");

        let gps_uart = UartPeripheral::new(
            UART1,
            (pins.gpio20.into_function(), pins.gpio21.into_function()),
            &mut RESETS,
        )
        .enable(
            UartConfig::new(GPS_UART_BAUD.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .map_err(|_| "gps uart init failed")?;
        rprintln!("boot: gps uart initialized");

        let pwm_slices = hal::pwm::Slices::new(PWM, &mut RESETS);
        rprintln!("boot: pwm slices initialized");

        let mut esc_xy_slice = pwm_slices.pwm4;
        esc_xy_slice.default_config();
        esc_xy_slice.set_div_int(ESC_PWM_DIV_INT);
        esc_xy_slice.set_div_frac(0);
        esc_xy_slice.set_top(esc_pwm_top);
        esc_xy_slice.set_counter(0);
        esc_xy_slice.enable();

        let mut esc_z_slice = pwm_slices.pwm5;
        esc_z_slice.default_config();
        esc_z_slice.set_div_int(ESC_PWM_DIV_INT);
        esc_z_slice.set_div_frac(0);
        esc_z_slice.set_top(esc_pwm_top);
        esc_z_slice.set_counter(0);
        esc_z_slice.enable();

        let mut test_slice = pwm_slices.pwm7;
        test_slice.default_config();
        test_slice.set_div_int(ESC_PWM_DIV_INT);
        test_slice.set_div_frac(0);
        test_slice.set_top(esc_pwm_top);
        test_slice.set_counter(0);
        test_slice.enable();

        let mut esc_x_channel = esc_xy_slice.channel_a;
        esc_x_channel.set_enabled(true);
        let esc_x_pin = esc_x_channel.output_to(
            pins.gpio8.into_function::<hal::gpio::FunctionPwm>(),
        );
        let mut esc_x: BldcX = BldcEsc::new(esc_x_channel, esc_x_pin, NoPowerGate);
        esc_x.enable();
        esc_x.set_duty_cycle(Self::pulse_width_to_duty(esc_pwm_top, ESC_MIN_PULSE_WIDTH_US as u16));

        let mut esc_y_channel = esc_xy_slice.channel_b;
        esc_y_channel.set_enabled(true);
        let esc_y_pin = esc_y_channel.output_to(
            pins.gpio9.into_function::<hal::gpio::FunctionPwm>(),
        );
        let mut esc_y: BldcY = BldcEsc::new(esc_y_channel, esc_y_pin, NoPowerGate);
        esc_y.enable();
        esc_y.set_duty_cycle(Self::pulse_width_to_duty(esc_pwm_top, ESC_MIN_PULSE_WIDTH_US as u16));

        let mut esc_z_channel = esc_z_slice.channel_a;
        esc_z_channel.set_enabled(true);
        let esc_z_pin = esc_z_channel.output_to(
            pins.gpio10.into_function::<hal::gpio::FunctionPwm>(),
        );
        let mut esc_z: BldcZ = BldcEsc::new(esc_z_channel, esc_z_pin, NoPowerGate);
        esc_z.enable();
        esc_z.set_duty_cycle(Self::pulse_width_to_duty(esc_pwm_top, ESC_MIN_PULSE_WIDTH_US as u16));

        let mut test_max_channel = test_slice.channel_a;
        test_max_channel.set_enabled(true);
        let test_max_pin = test_max_channel.output_to(
            pins.gpio14.into_function::<hal::gpio::FunctionPwm>(),
        );
        let mut test_max_esc = BldcEsc::new(test_max_channel, test_max_pin, NoPowerGate);
        test_max_esc.enable();
        test_max_esc.set_duty_cycle(Self::pulse_width_to_duty(
            esc_pwm_top,
            TEST_MAX_PWM_PULSE_WIDTH_US,
        ));

        let mut test_min_channel = test_slice.channel_b;
        test_min_channel.set_enabled(true);
        let test_min_pin = test_min_channel.output_to(
            pins.gpio15.into_function::<hal::gpio::FunctionPwm>(),
        );
        let mut test_min_esc = BldcEsc::new(test_min_channel, test_min_pin, NoPowerGate);
        test_min_esc.enable();
        test_min_esc.set_duty_cycle(Self::pulse_width_to_duty(
            esc_pwm_top,
            TEST_MIN_PWM_PULSE_WIDTH_US,
        ));

        rprintln!("satellite board initialized");

        Ok(Self {
            led,
            _test_max_esc: test_max_esc,
            _test_min_esc: test_min_esc,
            _shared_i2c: shared_i2c,
            bmi323,
            bmm350,
            delay,
            gps_uart,
            compute_uart,
            esc_x,
            esc_y,
            esc_z,
            telemetry_seq: 0,
            compute_rx_count: 0,
            compute_line: String::new(),
            esc_pulse_widths_us: [ESC_MIN_PULSE_WIDTH_US as u16; 3],
            esc_pwm_top,
            sample_debug_budget: 6,
            system_clock_hz,
        })
    }

    pub(super) fn system_clock_hz(&self) -> u32 {
        self.system_clock_hz
    }

    fn init_bmi323(
        shared_i2c: &'static SatelliteI2cArbiter,
        delay: SensorDelay,
    ) -> Option<BmiDevice> {
        let mut bmi323 = BmiDevice::new_with_i2c(ArbiterDevice::new(shared_i2c), BMI323_I2C_ADDR, delay);
        if bmi323.init().is_err() {
            return None;
        }

        let accel_config = AccelConfig::builder()
            .odr(OutputDataRate::Odr100hz)
            .range(AccelerometerRange::G8)
            .bw(Bandwidth::OdrQuarter)
            .avg_num(AverageNum::Avg4)
            .mode(AccelerometerPowerMode::Normal)
            .build();
        if bmi323.set_accel_config(accel_config).is_err() {
            return None;
        }

        let gyro_config = GyroConfig::builder()
            .odr(OutputDataRate::Odr100hz)
            .range(GyroscopeRange::DPS2000)
            .bw(Bandwidth::OdrHalf)
            .avg_num(AverageNum::Avg4)
            .mode(GyroscopePowerMode::Normal)
            .build();
        if bmi323.set_gyro_config(gyro_config).is_err() {
            return None;
        }

        rprintln!("boot: bmi323 initialized");
        Some(bmi323)
    }

    fn init_bmm350(
        shared_i2c: &'static SatelliteI2cArbiter,
        delay: SensorDelay,
    ) -> Option<BmmDevice> {
        let mut bmm350 = BmmDevice::new_with_i2c(ArbiterDevice::new(shared_i2c), BMM350_I2C_ADDR, delay);
        if bmm350.init().is_err() {
            return None;
        }

        if bmm350
            .enable_axes(
                AxisEnableDisable::Enable,
                AxisEnableDisable::Enable,
                AxisEnableDisable::Enable,
            )
            .is_err()
        {
            return None;
        }

        if bmm350
            .set_mag_config(
                MagConfig::builder()
                    .odr(DataRate::ODR25Hz)
                    .performance(PerformanceMode::Regular)
                    .mode(PowerMode::Normal)
                    .build(),
            )
            .is_err()
        {
            return None;
        }

        if bmm350.set_power_mode(PowerMode::Normal).is_err() {
            return None;
        }

        let _ = BmmAverageNum::Avg4;
        rprintln!("boot: bmm350 initialized");
        Some(bmm350)
    }

    fn debug_sample_stage(&mut self, message: &str) {
        if self.sample_debug_budget > 0 {
            rprintln!("sample: {}", message);
        }
    }

    fn finish_sample_debug(&mut self) {
        if self.sample_debug_budget > 0 {
            self.sample_debug_budget -= 1;
        }
    }

    fn pulse_width_micros_value(pulse_width_us: u16) -> f32 {
        pulse_width_us as f32
    }

    pub(super) fn next_snapshot(&mut self, gps: GpsFix) -> TelemetrySnapshot {
        self.telemetry_seq = self.telemetry_seq.wrapping_add(1);
        self.debug_sample_stage("begin next_snapshot");
        let timestamp_ms = (<pulsar::arch::arm::ArmArch as Arch>::now_ns() / 1_000_000) as u32;
        let mut snapshot = TelemetrySnapshot {
            seq: self.telemetry_seq,
            timestamp_ms,
            compute_rx_count: self.compute_rx_count,
            motor_x: Self::pulse_width_micros_value(self.esc_pulse_widths_us[0]),
            motor_y: Self::pulse_width_micros_value(self.esc_pulse_widths_us[1]),
            motor_z: Self::pulse_width_micros_value(self.esc_pulse_widths_us[2]),
            ..TelemetrySnapshot::default()
        };

        self.debug_sample_stage("imu sample start");
        if let Some(imu) = self.bmi323.as_mut() {
            let accel = imu.read_accel_data_scaled();
            let gyro = imu.read_gyro_data_scaled();

            if let (Ok(accel), Ok(gyro)) = (accel, gyro) {
                snapshot.flags |= IMU_VALID_FLAG;
                snapshot.accel_x_g = accel.x / 9.80665;
                snapshot.accel_y_g = accel.y / 9.80665;
                snapshot.accel_z_g = accel.z / 9.80665;
                snapshot.gyro_x_dps = gyro.x;
                snapshot.gyro_y_dps = gyro.y;
                snapshot.gyro_z_dps = gyro.z;
                self.debug_sample_stage("imu sample ok");
            } else {
                rprintln!("bmi323 sample failed");
            }
        } else {
            self.debug_sample_stage("imu unavailable");
        }

        self.debug_sample_stage("mag sample start");
        if let Some(mag) = self.bmm350.as_mut() {
            if mag.read_mag_data().is_ok() {
                snapshot.flags |= MAG_VALID_FLAG;
                self.debug_sample_stage("mag sample ok");
            } else {
                rprintln!("bmm350 sample failed");
            }
        } else {
            self.debug_sample_stage("mag unavailable");
        }

        if gps.valid {
            snapshot.flags |= GPS_VALID_FLAG;
        }
        if self.compute_rx_count > 0 {
            snapshot.flags |= COMPUTE_LINK_FLAG;
        }

        self.debug_sample_stage("snapshot complete");
        self.finish_sample_debug();

        snapshot
    }

    pub(super) fn drain_gps<F>(&mut self, mut on_byte: F)
    where
        F: FnMut(u8),
    {
        let mut byte = [0u8; 1];
        while self.gps_uart.uart_is_readable() {
            match self.gps_uart.read_raw(&mut byte) {
                Ok(1) => on_byte(byte[0]),
                Ok(_) => break,
                Err(nb::Error::WouldBlock) => break,
                Err(_) => break,
            }
        }
    }

    pub(super) fn drain_compute_uart<F>(&mut self, mut on_message: F)
    where
        F: FnMut(ComputeMessage) -> bool,
    {
        let mut byte = [0u8; 1];
        while self.compute_uart.uart_is_readable() {
            match self.compute_uart.read_raw(&mut byte) {
                Ok(1) => self.handle_compute_byte(byte[0], &mut on_message),
                Ok(_) => break,
                Err(nb::Error::WouldBlock) => break,
                Err(_) => break,
            }
        }
    }

    fn handle_compute_byte<F>(&mut self, byte: u8, on_message: &mut F)
    where
        F: FnMut(ComputeMessage) -> bool,
    {
        match byte {
            b'\r' => {}
            b'\n' => {
                if !self.compute_line.is_empty() {
                    let line = self.compute_line.clone();
                    self.compute_line.clear();
                    self.handle_compute_line(line.as_str(), on_message);
                }
            }
            other => {
                if self.compute_line.len() < self.compute_line.capacity() {
                    let _ = self.compute_line.push(other as char);
                } else {
                    self.compute_line.clear();
                }
            }
        }
    }

    fn handle_compute_line<F>(&mut self, line: &str, on_message: &mut F)
    where
        F: FnMut(ComputeMessage) -> bool,
    {
        self.compute_rx_count = self.compute_rx_count.wrapping_add(1);

        let message = Self::parse_compute_line(line);
        if !on_message(message) {
            let _ = writeln!(self.compute_uart, "ERR mailbox_full=compute");
        }
    }

    fn parse_compute_line(line: &str) -> ComputeMessage {
        if let Some(seq) = line.strip_prefix("PING ") {
            let mut payload = String::<32>::new();
            let _ = payload.push_str(seq.trim());
            return ComputeMessage::Ping(payload);
        }

        if line == "SNAPSHOT" {
            return ComputeMessage::Snapshot;
        }

        if let Some(value) = line.strip_prefix("LED ") {
            return ComputeMessage::SetLed(value.trim() == "1");
        }

        if let Some(payload) = line.strip_prefix("MOTORS ") {
            let mut parts = payload.split_ascii_whitespace();
            let Some(x) = parts.next().and_then(Self::parse_pulse_width_micros) else {
                let mut text = String::<96>::new();
                let _ = text.push_str(line);
                return ComputeMessage::Unknown(text);
            };
            let Some(y) = parts.next().and_then(Self::parse_pulse_width_micros) else {
                let mut text = String::<96>::new();
                let _ = text.push_str(line);
                return ComputeMessage::Unknown(text);
            };
            let Some(z) = parts.next().and_then(Self::parse_pulse_width_micros) else {
                let mut text = String::<96>::new();
                let _ = text.push_str(line);
                return ComputeMessage::Unknown(text);
            };

            if parts.next().is_none() {
                return ComputeMessage::SetMotors([x, y, z]);
            }
        }

        if let Some(payload) = line.strip_prefix("ECHO ") {
            let mut text = String::<96>::new();
            let _ = text.push_str(payload);
            return ComputeMessage::Echo(text);
        }

        let mut text = String::<96>::new();
        let _ = text.push_str(line);
        ComputeMessage::Unknown(text)
    }

    fn parse_pulse_width_micros(value: &str) -> Option<u16> {
        let parsed = value.parse::<u16>().ok()?;
        Some(parsed.clamp(ESC_MIN_PULSE_WIDTH_US as u16, ESC_MAX_PULSE_WIDTH_US as u16))
    }

    fn current_uptime_ms() -> u32 {
        (<pulsar::arch::arm::ArmArch as Arch>::now_ns() / 1_000_000) as u32
    }

    fn pulse_width_to_duty(esc_pwm_top: u16, pulse_width_us: u16) -> u16 {
        (((esc_pwm_top as u32 + 1) * pulse_width_us as u32) / 20_000) as u16
    }

    fn set_esc_pulse_widths(&mut self, pulse_widths_us: [u16; 3]) {
        self.esc_x
            .set_duty_cycle(Self::pulse_width_to_duty(self.esc_pwm_top, pulse_widths_us[0]));
        self.esc_y
            .set_duty_cycle(Self::pulse_width_to_duty(self.esc_pwm_top, pulse_widths_us[1]));
        self.esc_z
            .set_duty_cycle(Self::pulse_width_to_duty(self.esc_pwm_top, pulse_widths_us[2]));
        self.esc_pulse_widths_us = pulse_widths_us;
    }

    fn gnc_spinup_phase() -> GncEscStartupPhase {
        let uptime_ms = Self::current_uptime_ms();
        if uptime_ms <= GNC_ESC_STOP_HOLD_MS {
            return GncEscStartupPhase::Stop;
        }

        let neutral_ramp_elapsed_ms = uptime_ms - GNC_ESC_STOP_HOLD_MS;
        if neutral_ramp_elapsed_ms <= GNC_ESC_NEUTRAL_RAMP_DURATION_MS {
            let min_pulse = ESC_MIN_PULSE_WIDTH_US as i32;
            let target_pulse = GNC_ESC_STARTUP_SWEEP_TARGET_US as i32;
            let pulse_width_us = min_pulse
                + ((target_pulse - min_pulse) * neutral_ramp_elapsed_ms as i32)
                    / GNC_ESC_NEUTRAL_RAMP_DURATION_MS as i32;
            return GncEscStartupPhase::PulseSweep {
                pulse_width_us: pulse_width_us as u16,
            };
        }

        let ramp_elapsed_ms =
            uptime_ms - GNC_ESC_STOP_HOLD_MS - GNC_ESC_NEUTRAL_RAMP_DURATION_MS;

        GncEscStartupPhase::Active {
            trim_scale_ms: ramp_elapsed_ms.min(GNC_ESC_TRIM_RAMP_DURATION_MS),
        }
    }

    fn set_all_escs_stop(&mut self) {
        self.set_esc_pulse_widths([ESC_MIN_PULSE_WIDTH_US as u16; 3]);
    }

    fn set_all_escs_neutral(&mut self) {
        self.set_esc_pulse_widths([ESC_NEUTRAL_PULSE_WIDTH_US as u16; 3]);
    }

    fn set_all_escs_pulse_width(&mut self, pulse_width_us: u16) {
        self.set_esc_pulse_widths([pulse_width_us; 3]);
    }

    pub(super) fn apply_motor_commands(&mut self, pulse_widths_us: [u16; 3]) {
        self.set_esc_pulse_widths(pulse_widths_us);
    }

    pub(super) fn apply_gnc_motor_commands(&mut self, trims: [i16; 3]) -> [f32; 3] {
        match Self::gnc_spinup_phase() {
            GncEscStartupPhase::Stop => {
                self.set_all_escs_stop();
                self.esc_pulse_widths_us.map(Self::pulse_width_micros_value)
            }
            GncEscStartupPhase::PulseSweep { pulse_width_us } => {
                self.set_all_escs_pulse_width(pulse_width_us);
                self.esc_pulse_widths_us.map(Self::pulse_width_micros_value)
            }
            GncEscStartupPhase::Active { trim_scale_ms } => {
                let applied = trims.map(|trim| {
                    if trim_scale_ms >= GNC_ESC_TRIM_RAMP_DURATION_MS {
                        trim as i32
                    } else {
                        (trim as i32 * trim_scale_ms as i32)
                            / GNC_ESC_TRIM_RAMP_DURATION_MS as i32
                    }
                    .clamp(-500, 500) as i16
                });
                self.esc_pulse_widths_us = [
                    BldcX::pulse_width_for_signed_pulse_offset_micros(applied[0]),
                    BldcY::pulse_width_for_signed_pulse_offset_micros(applied[1]),
                    BldcZ::pulse_width_for_signed_pulse_offset_micros(applied[2]),
                ];
                self.set_esc_pulse_widths(self.esc_pulse_widths_us);
                self.esc_pulse_widths_us.map(Self::pulse_width_micros_value)
            }
        }
    }

    pub(super) fn handle_compute_message(
        &mut self,
        message: ComputeMessage,
        latest: &TelemetryFrame,
        telemetry_channel: &mut UpChannel,
    ) {
        match message {
            ComputeMessage::Ping(seq) => {
                let _ = writeln!(self.compute_uart, "ACK {} seq={}", seq.as_str(), latest.snapshot.seq);
            }
            ComputeMessage::Snapshot => self.write_telemetry(latest, telemetry_channel),
            ComputeMessage::SetLed(enabled) => {
                if enabled {
                    let _ = self.led.set_high();
                    let _ = writeln!(self.compute_uart, "ACK led=1");
                } else {
                    let _ = self.led.set_low();
                    let _ = writeln!(self.compute_uart, "ACK led=0");
                }
            }
            ComputeMessage::SetMotors(pulse_widths_us) => {
                self.apply_motor_commands(pulse_widths_us);
                let _ = writeln!(
                    self.compute_uart,
                    "ACK motors={} {} {}",
                    pulse_widths_us[0],
                    pulse_widths_us[1],
                    pulse_widths_us[2]
                );
            }
            ComputeMessage::Echo(payload) => {
                let _ = writeln!(self.compute_uart, "ECHO {}", payload.as_str());
            }
            ComputeMessage::Unknown(line) => {
                let _ = writeln!(self.compute_uart, "ERR unknown_command={}", line.as_str());
            }
        }
    }

    pub(super) fn write_telemetry(&mut self, frame: &TelemetryFrame, telemetry_channel: &mut UpChannel) {
        let snapshot = frame.snapshot;
        let mut line: String<1024> = String::new();
        if writeln!(
            line,
            "telemetry,seq={},timestamp_ms={},flags={},phase={},maneuver_stage={},accel_x_g={:.4},accel_y_g={:.4},accel_z_g={:.4},gyro_x_dps={:.4},gyro_y_dps={:.4},gyro_z_dps={:.4},curr_roll={:.6},curr_pitch={:.6},curr_yaw={:.6},curr_rate_x={:.6},curr_rate_y={:.6},curr_rate_z={:.6},desired_roll={:.6},desired_pitch={:.6},desired_yaw={:.6},desired_rate_x={:.6},desired_rate_y={:.6},desired_rate_z={:.6},torque_x={:.6},torque_y={:.6},torque_z={:.6},motor_x={:.2},motor_y={:.2},motor_z={:.2},compute_rx_count={}",
            snapshot.seq,
            snapshot.timestamp_ms,
            snapshot.flags,
            frame.phase as u8,
            frame.gnc.maneuver_stage,
            snapshot.accel_x_g,
            snapshot.accel_y_g,
            snapshot.accel_z_g,
            snapshot.gyro_x_dps,
            snapshot.gyro_y_dps,
            snapshot.gyro_z_dps,
            frame.gnc.curr_roll,
            frame.gnc.curr_pitch,
            frame.gnc.curr_yaw,
            frame.gnc.curr_rate_x,
            frame.gnc.curr_rate_y,
            frame.gnc.curr_rate_z,
            frame.gnc.desired_roll,
            frame.gnc.desired_pitch,
            frame.gnc.desired_yaw,
            frame.gnc.desired_rate_x,
            frame.gnc.desired_rate_y,
            frame.gnc.desired_rate_z,
            frame.gnc.torque_x,
            frame.gnc.torque_y,
            frame.gnc.torque_z,
            snapshot.motor_x,
            snapshot.motor_y,
            snapshot.motor_z,
            snapshot.compute_rx_count,
        )
        .is_err()
        {
            rprintln!("telemetry format overflow seq={}", snapshot.seq);
            return;
        }
        let _ = telemetry_channel.write_str(line.as_str());
        rprintln!("{}", line.as_str());
    }
}