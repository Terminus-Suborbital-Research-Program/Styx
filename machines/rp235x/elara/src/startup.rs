#![warn(missing_docs, clippy::unwrap_used)]
use crate::{
    actuators::servo::Servo,
    device_constants::{
        pins::{MuxEPin, MuxS0Pin, MuxS1Pin, MuxS2Pin, MuxS3Pin},
        ComputeRXBuffer, ComputeTXBuffer, OdinComputeUart,
    },
};
use crate::{
    app::*,
    device_constants::pins::{AvionicsI2CSclPin, AvionicsI2CSdaPin, EscI2CSclPin, EscI2CSdaPin},
    peripherals::async_i2c::AsyncI2c,
    Mono,
};
use defmt::{info, panic, warn};
use defmt_rtt as _;
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
};

use embedded_hal_0_2::blocking::i2c::Write;

use fugit::RateExtU32;
use hc12_rs::{
    configuration::{baudrates::B9600, Channel, HC12Configuration, Power},
    device::{IntoATMode, IntoFU3Mode},
};
use rp235x_hal::{
    clocks,
    gpio::{FunctionI2C, FunctionPwm, FunctionUartAux, Pin, PullNone, PullUp},
    pwm::Slices,
    uart::{DataBits, StopBits, UartConfig, UartPeripheral},
    Clock, Sio, Watchdog, I2C,
};
use rp235x_hal::adc::AdcPin;
use rtic_sync::arbiter::{i2c::ArbiterDevice, Arbiter};

// Sensors
// use crate::device_constants::IcarusHC12;
use bme280::AsyncBME280;
use bmi323::AsyncBmi323;
use bmm350::AsyncBmm350;

use crate::pdmux_controller::PDMuxController;

// Logs our time for demft
defmt::timestamp!("{=u64:us}", { epoch_ns() });
pub fn startup(mut ctx: init::Context) -> (Shared, Local) {
    // Set up clocks
    let mut watchdog = Watchdog::new(ctx.device.WATCHDOG);

    // info!("Good morning sunshine! Icarus is awake!");

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

    Mono::start(ctx.core.SYST, clocks.system_clock.freq().to_Hz());

    let compute_link: OdinComputeUart = UartPeripheral::new(
        ctx.device.UART1,
        (
            pins.gpio38.into_function::<FunctionUartAux>(),
            pins.gpio37.into_function(),
        ),
        &mut ctx.device.RESETS,
    )
    .enable(
        UartConfig::new(115200_u32.Hz(), DataBits::Eight, None, StopBits::One),
        clocks.peripheral_clock.freq(),
    )
    .unwrap();

    // Configure GPIO25 as an output
    let mut led_pin = pins
        .gpio25
        .into_pull_type::<PullNone>()
        .into_push_pull_output();
    led_pin.set_low().unwrap();

    // Sensors
    // Init I2C pins

    let avionics_sda_pin: Pin<AvionicsI2CSdaPin, FunctionI2C, PullUp> = pins.gpio4.reconfigure();
    let avionics_scl_pin: Pin<AvionicsI2CSclPin, FunctionI2C, PullUp> = pins.gpio5.reconfigure();

    let mut avionics_i2c = I2C::i2c0(
        ctx.device.I2C0,
        avionics_sda_pin,
        avionics_scl_pin,
        400.kHz(),
        &mut ctx.device.RESETS,
        &clocks.system_clock,
    );

    // Works
    // defmt::info!("Writing");
    // avionics_i2c.write(0x2Cu8, &[1, 2, 3]).unwrap();
    // defmt::info!("Written");

    let async_avionics_i2c = AsyncI2c::new(avionics_i2c, 10_u32);
    let avionics_i2c_arbiter = ctx
        .local
        .i2c_avionics_bus
        .write(Arbiter::new(async_avionics_i2c));

    // Initialize Avionics Sensors
    let bmm350 = AsyncBmm350::new_with_i2c(ArbiterDevice::new(avionics_i2c_arbiter), 0x14, Mono);
    let bmi323 = AsyncBmi323::new_with_i2c(ArbiterDevice::new(avionics_i2c_arbiter), 0x68, Mono);
    let bme280 = AsyncBME280::new(ArbiterDevice::new(avionics_i2c_arbiter), 0x77, Mono);


    // let mut adc = rp235x_hal::Adc::new(ctx.device.ADC, &mut ctx.device.RESETS);
    // let mut adc_photoresistors: rp235x_hal::adc::AdcPin<Pin<rp235x_hal::gpio::bank0::Gpio40, rp235x_hal::gpio::FunctionNull, rp235x_hal::gpio::PullDown>> = rp235x_hal::adc::AdcPin::new(pins.gpio40).unwrap();

    *ctx.local.adc = Some(rp235x_hal::Adc::new(ctx.device.ADC, &mut ctx.device.RESETS));
    let adc = ctx.local.adc.as_mut().unwrap();


    let adc_fifo = Some(adc_fifo);

    let data = ComputeTXBuffer::new();
    let metrics_buf = ComputeRXBuffer::new();

    info!("Peripherals initialized, spawning tasks...");
    heartbeat::spawn().ok();
    sample_sensors::spawn(avionics_i2c_arbiter).ok();
    info!("Tasks spawned!");
    (
        Shared { data, metrics_buf },
        Local {
            led: led_pin,
            bmm350,
            bmi323,
            bme280,
            ina260_1,
            ina260_2,
            ina260_3,
            rbf,
            ina260_4,
            pd_mux: PDMuxController::new(
                pins.gpio19.into_pull_type::<PullNone>().into_push_pull_output(), // S0
                pins.gpio20.into_pull_type::<PullNone>().into_push_pull_output(), // S1
                pins.gpio21.into_pull_type::<PullNone>().into_push_pull_output(), // S2
                pins.gpio12.into_pull_type::<PullNone>().into_push_pull_output(), // Disable pin
                AdcPin::new(pins.gpio14.into_floating_input()).unwrap(), // Input 0
                AdcPin::new(pins.gpio13.into_floating_input()).unwrap(), // Input 1
                AdcPin::new(pins.gpio11.into_floating_input()).unwrap(), // Input 2
                AdcPin::new(pins.gpio10.into_floating_input()).unwrap(), // Input 3
                adc
            ),
            compute_link,

        },
    )
}
