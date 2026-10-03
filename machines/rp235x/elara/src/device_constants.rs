#![warn(missing_docs)]

use actuators::bldc::BldcEsc;
use bmi323::Bmi323;
use bmm350::Bmm350;
use pulsar::sync::{Arbiter, ArbiterDevice};

#[cfg(all(target_arch = "arm", target_os = "none"))]
use rp235x_hal as hal;
#[cfg(all(target_arch = "arm", target_os = "none"))]
use rp235x_hal::Timer;

/// External crystal frequency.
pub(super) const XTAL_FREQ_HZ: u32 = 12_000_000;
/// CPU clock frequency used for simple delay loops.
pub(super) const CPU_HZ: u32 = 150_000_000;
/// Scheduler tick frequency.
pub(super) const TICK_HZ: u32 = 1_000;
/// Sensor and NGC loop rate.
pub(super) const SENSOR_RATE_HZ: u32 = 500;
/// Avionics I2C bus speed.
pub(super) const AVIONICS_I2C_HZ: u32 = 400_000;
/// Alias retained for the current Pulsar board bring-up path.
pub(super) const I2C_BAUD_HZ: u32 = AVIONICS_I2C_HZ;
/// ESC PWM divider integer.
pub(super) const ESC_PWM_DIV_INT: u8 = 64;
/// Maximum torque mapped to full actuator trim authority.
pub(super) const MAX_TORQUE_COMMAND_NM: f32 = 0.25;
/// GPS poll task rate.
pub(super) const GPS_POLL_HZ: u32 = 200;
/// Compute UART poll task rate.
pub(super) const COMPUTE_POLL_HZ: u32 = 20;
/// Sensor sampling task rate.
pub(super) const IMU_SAMPLE_HZ: u32 = SENSOR_RATE_HZ;
/// Telemetry output task rate.
pub(super) const TELEMETRY_HZ: u32 = 10;
/// GNC update task rate.
pub(super) const GNC_HZ: u32 = SENSOR_RATE_HZ;
/// Runtime gate for BMI323 sampling.
pub(super) const ENABLE_BMI323_RUNTIME_SAMPLING: bool = true;
/// Runtime gate for BMM350 sampling.
pub(super) const ENABLE_BMM350_RUNTIME_SAMPLING: bool = true;
/// BMI323 I2C address.
pub(super) const BMI323_I2C_ADDR: u8 = 0x68;
/// BMM350 I2C address.
pub(super) const BMM350_I2C_ADDR: u8 = 0x14;

/// Avionics I2C SDA GPIO.
pub(super) type AvionicsI2CSdaPin = hal::gpio::bank0::Gpio4;
/// Avionics I2C SCL GPIO.
pub(super) type AvionicsI2CSclPin = hal::gpio::bank0::Gpio5;
/// BLDC X PWM GPIO.
pub(super) type BldcXPwmPin = hal::gpio::bank0::Gpio8;
/// BLDC Y PWM GPIO.
pub(super) type BldcYPwmPin = hal::gpio::bank0::Gpio9;
/// BLDC Z PWM GPIO.
pub(super) type BldcZPwmPin = hal::gpio::bank0::Gpio10;
/// Compute UART TX GPIO.
pub(super) type ComputeTxPin = hal::gpio::bank0::Gpio0;
/// Compute UART RX GPIO.
pub(super) type ComputeRxPin = hal::gpio::bank0::Gpio1;
/// GPS UART TX GPIO.
pub(super) type GpsTxPin = hal::gpio::bank0::Gpio20;
/// GPS UART RX GPIO.
pub(super) type GpsRxPin = hal::gpio::bank0::Gpio21;
/// Bench test max PWM GPIO.
pub(super) type TestMaxPwmPin = hal::gpio::bank0::Gpio14;
/// Bench test min PWM GPIO.
pub(super) type TestMinPwmPin = hal::gpio::bank0::Gpio15;

/// Concrete avionics I2C controller type.
pub(super) type AvionicsI2cController = hal::I2C<
    hal::pac::I2C0,
    (
        hal::gpio::Pin<AvionicsI2CSdaPin, hal::gpio::FunctionI2C, hal::gpio::PullUp>,
        hal::gpio::Pin<AvionicsI2CSclPin, hal::gpio::FunctionI2C, hal::gpio::PullUp>,
    ),
    hal::i2c::Controller,
>;

/// Shared avionics I2C arbiter.
pub(super) type AvionicsI2cArbiter = Arbiter<AvionicsI2cController>;
/// Shared avionics I2C device view.
pub(super) type AvionicsI2cDevice = ArbiterDevice<'static, AvionicsI2cController>;
/// Alias retained for the current Pulsar board bring-up path.
pub(super) type SatelliteI2cController = AvionicsI2cController;
/// Alias retained for the current Pulsar board bring-up path.
pub(super) type SatelliteI2cArbiter = AvionicsI2cArbiter;
/// Timer used for synchronous sensor delays.
pub(super) type SensorDelay = Timer<hal::timer::CopyableTimer0>;
/// BMI323 device type.
pub(super) type BmiDevice =
    Bmi323<bmi323::interface::I2cInterface<AvionicsI2cDevice>, SensorDelay>;
/// BMM350 device type.
pub(super) type BmmDevice =
    Bmm350<bmm350::interface::I2cInterface<AvionicsI2cDevice>, SensorDelay>;
/// PWM slice used for BLDC X/Y.
pub(super) type BldcSlice4 = hal::pwm::Slice<hal::pwm::Pwm4, hal::pwm::FreeRunning>;
/// PWM slice used for BLDC Z.
pub(super) type BldcSlice5 = hal::pwm::Slice<hal::pwm::Pwm5, hal::pwm::FreeRunning>;
/// PWM slice used for the bench test outputs.
pub(super) type TestSlice7 = hal::pwm::Slice<hal::pwm::Pwm7, hal::pwm::FreeRunning>;
/// BLDC X ESC type.
pub(super) type BldcX = BldcEsc<
    hal::pwm::Channel<BldcSlice4, hal::pwm::A>,
    hal::gpio::Pin<BldcXPwmPin, hal::gpio::FunctionPwm, hal::gpio::PullDown>,
>;
/// BLDC Y ESC type.
pub(super) type BldcY = BldcEsc<
    hal::pwm::Channel<BldcSlice4, hal::pwm::B>,
    hal::gpio::Pin<BldcYPwmPin, hal::gpio::FunctionPwm, hal::gpio::PullDown>,
>;
/// BLDC Z ESC type.
pub(super) type BldcZ = BldcEsc<
    hal::pwm::Channel<BldcSlice5, hal::pwm::A>,
    hal::gpio::Pin<BldcZPwmPin, hal::gpio::FunctionPwm, hal::gpio::PullDown>,
>;
/// Alias retained for the current board abstraction.
pub(super) type EscX = BldcX;
/// Alias retained for the current board abstraction.
pub(super) type EscY = BldcY;
/// Alias retained for the current board abstraction.
pub(super) type EscZ = BldcZ;
/// Bench test max ESC type.
pub(super) type TestMaxEsc = BldcEsc<
    hal::pwm::Channel<TestSlice7, hal::pwm::A>,
    hal::gpio::Pin<TestMaxPwmPin, hal::gpio::FunctionPwm, hal::gpio::PullDown>,
>;
/// Bench test min ESC type.
pub(super) type TestMinEsc = BldcEsc<
    hal::pwm::Channel<TestSlice7, hal::pwm::B>,
    hal::gpio::Pin<TestMinPwmPin, hal::gpio::FunctionPwm, hal::gpio::PullDown>,
>;
