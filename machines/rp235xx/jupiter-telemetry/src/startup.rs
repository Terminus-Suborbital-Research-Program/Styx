use crate::{
    device_constants::{
        pins::{MuxEPin, MuxS0Pin, MuxS1Pin, MuxS2Pin, MuxS3Pin},
        DownlinkBuffer,
    },
};
use crate::{
    app::*,
    device_constants::{
        pins::{AvionicsI2CSclPin, AvionicsI2CSdaPin, EscI2CSclPin, EscI2CSdaPin},

    },
    peripherals::async_i2c::AsyncI2c,
    Mono,
};
use defmt::{error, info, warn};
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
};
use fugit::RateExtU32;

use rp235x_hal::{
    clocks,
    gpio::{FunctionI2C, FunctionPwm, Pin, PullNone, PullUp},
    pwm::Slices,
    uart::{DataBits, StopBits, UartConfig, UartPeripheral},
    Clock, Sio, Watchdog, I2C,
};
use rtic_sync::arbiter::{i2c::ArbiterDevice, Arbiter};

// Sensors
use bme280::AsyncBME280;
use bmi323::AsyncBmi323;
use bmm350::AsyncBmm350;
use bmp5::{Config as Bmp5Config, i2c::{Bmp5, BMP5_ADDRESS}};

use adxl345_eh_driver;

// Logs our time for demft
defmt::timestamp!("{=u64:us}", { epoch_ns() });

pub fn startup(mut ctx: init::Context) -> (Shared, Local) {
    // Reset the spinlocks - this is skipped by soft-reset
    unsafe {
        rp235x_hal::sio::spinlock_reset();
    }

    // Set up clocks
    let mut watchdog = Watchdog::new(ctx.device.WATCHDOG);

    info!("Good morning sunshine! Icarus is awake!");

    Mono::start(ctx.device.TIMER0, &ctx.device.RESETS);

    // The single-cycle I/O block controls our GPIO pins
    let sio = Sio::new(ctx.device.SIO);

    // Set the pins to their default state
    let pins = rp235x_hal::gpio::Pins::new(
        ctx.device.IO_BANK0,
        ctx.device.PADS_BANK0,
        sio.gpio_bank0,
        &mut ctx.device.RESETS,
    );
    // let mut debug_pin = pins.gpio11.into_push_pull_output();
    // debug_pin.set_high().unwrap();
    let clocks = match clocks::init_clocks_and_plls(
        12_000_000u32,
        ctx.device.XOSC,
        ctx.device.CLOCKS,
        ctx.device.PLL_SYS,
        ctx.device.PLL_USB,
        &mut ctx.device.RESETS,
        &mut watchdog,
    ) {
        Ok(clocks) => clocks,
        Err(e) => {
            // Debug pin
            if match e {
                clocks::InitError::XoscErr(_) => false,
                clocks::InitError::PllError(_) => false,
                clocks::InitError::ClockError(_) => false,
            } {
                // debug_pin.set_high().unwrap();
            } else {
                // debug_pin.set_low().unwrap();
            }
            warn!("Failed to init clocks: {:?}", e);
            panic!("Failed to init clocks");
        }
    };

    // Configure GPIO25 as an output
    let mut led_pin = pins
        .gpio25
        .into_pull_type::<PullNone>()
        .into_push_pull_output();
    led_pin.set_low().unwrap();

    // Pin setup for UART1
    let uart1_pins = (pins.gpio8.into_function(), pins.gpio9.into_function());
    let mut uart1_peripheral =
        UartPeripheral::new(ctx.device.UART1, uart1_pins, &mut ctx.device.RESETS)
            .enable(
                UartConfig::new(9600_u32.Hz(), DataBits::Eight, None, StopBits::One),
                clocks.peripheral_clock.freq(),
            )
            .unwrap();
    uart1_peripheral.enable_rx_interrupt(); // Make sure we can drive our interrupts

    let programming = pins.gpio5.into_push_pull_output();
    // Copy the timer
    let timer = rp235x_hal::Timer::new_timer1(ctx.device.TIMER1, &mut ctx.device.RESETS, &clocks);
    let mut timer_two = timer;

    
    // Sensors
    // Init I2C pins
    let compute_sda_pin: Pin<EscI2CSdaPin, FunctionI2C, PullUp> = pins.gpio16.reconfigure();
    let compute_scl_pin: Pin<EscI2CSclPin, FunctionI2C, PullUp> = pins.gpio17.reconfigure();

    let compute_i2c = I2C::new_controller(
        ctx.device.I2C0,
        compute_sda_pin,
        compute_scl_pin,
        RateExtU32::kHz(400),
        &mut ctx.device.RESETS,
        clocks.system_clock.freq(),
    );

    // let mut accel = adxl345_eh_driver::Driver::new(compute_i2c, None).unwrap();
    // let (x, y, z) = accel.get_accel_raw().unwrap();


    let async_compute_i2c = AsyncI2c::new(compute_i2c, 10);
    let compute_i2c_arbiter = ctx.local.i2c_compute_bus.write(Arbiter::new(async_compute_i2c));

    let avionics_sda_pin: Pin<AvionicsI2CSdaPin, FunctionI2C, PullUp> = pins.gpio6.reconfigure();
    let avionics_scl_pin: Pin<AvionicsI2CSclPin, FunctionI2C, PullUp> = pins.gpio7.reconfigure();

    let avionics_i2c = I2C::new_controller(
        ctx.device.I2C1,
        avionics_sda_pin,
        avionics_scl_pin,
        RateExtU32::kHz(400),
        &mut ctx.device.RESETS,
        clocks.system_clock.freq(),
    );

    let async_avionics_i2c = AsyncI2c::new(avionics_i2c, 10_u32);
    let avionics_i2c_arbiter = ctx
        .local
        .i2c_avionics_bus
        .write(Arbiter::new(async_avionics_i2c));

    // let mut delay_here = hal::Timer::new_timer1(pac.TIMER1, &mut pac.RESETS, &clocks);

    // Initialize Avionics Sensors
    let bmm350 = AsyncBmm350::new_with_i2c(ArbiterDevice::new(avionics_i2c_arbiter), 0x14, Mono);
    let bmi323 = AsyncBmi323::new_with_i2c(ArbiterDevice::new(avionics_i2c_arbiter), 0x69, Mono);
    let bme280 = AsyncBME280::new(ArbiterDevice::new(avionics_i2c_arbiter), 0x77, Mono);
    let mut bmp5 = Bmp5::new(ArbiterDevice::new(avionics_i2c_arbiter), Mono, BMP5_ADDRESS, Bmp5Config::default());


    let data = DownlinkBuffer::new();

    info!("Peripherals initialized, spawning tasks...");
    heartbeat::spawn().ok();
    sample_sensors::spawn(avionics_i2c_arbiter).ok();
    info!("Tasks spawned!");
    (
        Shared { data },
        Local {
            led: led_pin,
            bmm350,
            bmi323,
            bme280,
            bmp5,
        },
    )
}
