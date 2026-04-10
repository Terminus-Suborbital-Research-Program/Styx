//! # I²C Example
//!
//! This application demonstrates how to talk to I²C devices with an rp235x.
//!
//! It may need to be adapted to your particular board layout and/or pin assignment.
//!
//! See the `Cargo.toml` file for Copyright and license details.

#![no_std]
#![no_main]

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use cortex_m::{asm, delay::Delay as CortexDelay};
use panic_halt as _;
use rtt_target::ChannelMode::NoBlockSkip;
use rtt_target::{rprintln, rtt_init, set_print_channel};


// Alias for our HAL crate
use rp235x_hal as hal;

// Some things we need
use hal::{
    fugit::RateExtU32,
    gpio::{FunctionI2C, Pin},
    Clock,
};
use bmi323::{AccelConfig, AccelerometerRange, Bmi323, GyroConfig, GyroscopeRange, OutputDataRate};

/// Tell the Boot ROM about our application
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// External high-speed crystal on the Raspberry Pi Pico 2 board is 12 MHz.
/// Adjust if your board has a different frequency
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

struct EmbeddedDelay(CortexDelay);

impl embedded_hal::delay::DelayNs for EmbeddedDelay {
    fn delay_ns(&mut self, ns: u32) {
        let us = ns.div_ceil(1_000).max(1);
        self.0.delay_us(us);
    }

    fn delay_us(&mut self, us: u32) {
        self.0.delay_us(us.max(1));
    }

    fn delay_ms(&mut self, ms: u32) {
        self.0.delay_ms(ms.max(1));
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

/// Entry point to our bare-metal application.
///
/// The `#[hal::entry]` macro ensures the Cortex-M start-up code calls this function
/// as soon as all global variables and the spinlock are initialised.
///
/// The function configures the rp235x peripherals, then performs a single I²C
/// write to a fixed address.
#[hal::entry]
fn main() -> ! {
    init_logs();
    rprintln!("print channel ready");

    let core = cortex_m::Peripherals::take().unwrap();
    let mut pac = hal::pac::Peripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
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

    let system_clock_hz = clocks.system_clock.freq().to_Hz();
    let delay = EmbeddedDelay(CortexDelay::new(core.SYST, system_clock_hz));

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);

    // Set the pins to their default state
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Configure two pins as being I²C, not GPIO
    let sda_pin: Pin<_, FunctionI2C, _> = pins.gpio4.reconfigure();
    let scl_pin: Pin<_, FunctionI2C, _> = pins.gpio5.reconfigure();
    // let not_an_scl_pin: Pin<_, FunctionI2C, PullUp> = pins.gpio20.reconfigure();

    // Create the I²C drive, using the two pre-configured pins. This will fail
    // at compile time if the pins are in the wrong mode, or if this I²C
    // peripheral isn't available on these pins!
    let i2c = hal::I2C::i2c0(
        pac.I2C0,
        sda_pin,
        scl_pin, // Try `not_an_scl_pin` here
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let mut bmi323: Bmi323<_, _> = Bmi323::new_with_i2c(i2c, 0x68, delay);

    rprintln!("Initializing BMI323 at 0x{:02x}", 0x68);
    match bmi323.init() {
        Ok(()) => rprintln!("BMI323 init ok"),
        Err(err) => {
            rprintln!("BMI323 init failed: {:?}", err);
            loop {
                hal::arch::wfi();
            }
        }
    }

    let accel_config = AccelConfig::builder()
        .odr(OutputDataRate::Odr100hz)
        .range(AccelerometerRange::G8)
        .build();
    if let Err(err) = bmi323.set_accel_config(accel_config) {
        rprintln!("BMI323 accel config failed: {:?}", err);
    }

    let gyro_config = GyroConfig::builder()
        .odr(OutputDataRate::Odr100hz)
        .range(GyroscopeRange::DPS2000)
        .build();
    if let Err(err) = bmi323.set_gyro_config(gyro_config) {
        rprintln!("BMI323 gyro config failed: {:?}", err);
    }

    rprintln!("BMI323 configured");

    // Demo finish - just loop until reset

    loop {
        match bmi323.read_accel_data_scaled() {
            Ok(accel) => rprintln!(
                "accel m/s^2 => x: {:?}, y: {:?}, z: {:?}",
                accel.x,
                accel.y,
                accel.z
            ),
            Err(err) => rprintln!("BMI323 accel read failed: {:?}", err),
        }

        match bmi323.read_gyro_data_scaled() {
            Ok(gyro) => rprintln!(
                "gyro dps => x: {:?}, y: {:?}, z: {:?}",
                gyro.x,
                gyro.y,
                gyro.z
            ),
            Err(err) => rprintln!("BMI323 gyro read failed: {:?}", err),
        }

        asm::delay(system_clock_hz / 8);
    }
}

/// Program metadata for `picotool info`
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    hal::binary_info::rp_cargo_bin_name!(),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"I²C Example"),
    hal::binary_info::rp_cargo_homepage_url!(),
    hal::binary_info::rp_program_build_attribute!(),
];

// End of file