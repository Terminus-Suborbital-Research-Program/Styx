use crate::{
    actuators::servo::Servo,
    device_constants::{
        ComputeRXBuffer, ComputeTXBuffer, OdinComputeUart, pins::{MuxEPin, MuxS0Pin, MuxS1Pin, MuxS2Pin, MuxS3Pin}
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
use hc12_rs::{
    configuration::{baudrates::B9600, Channel, HC12Configuration, Power},
    device::{IntoATMode, IntoFU3Mode},
};
use rp235x_hal::{
    clocks,
    gpio::{FunctionI2C, FunctionPwm, Pin, PullNone, PullUp, FunctionUartAux},
    pwm::Slices,
    uart::{DataBits, StopBits, UartConfig, UartPeripheral},
    Clock, Sio, Watchdog, I2C,
};
use rtic_sync::arbiter::{i2c::ArbiterDevice, Arbiter};

// Sensors
// use crate::device_constants::IcarusHC12;
use crate::device_constants::MpChannel;
use bme280::AsyncBME280;
use bmi323::AsyncBmi323;
use bmm350::AsyncBmm350;
use cd74hc4067::CD74HC4067;
use ina260_terminus::AsyncINA260;

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
    let motor_sda_pin: Pin<EscI2CSdaPin, FunctionI2C, PullUp> = pins.gpio16.reconfigure();
    let motor_scl_pin: Pin<EscI2CSclPin, FunctionI2C, PullUp> = pins.gpio17.reconfigure();

    let motor_i2c = I2C::new_controller(
        ctx.device.I2C0,
        motor_sda_pin,
        motor_scl_pin,
        RateExtU32::kHz(400),
        &mut ctx.device.RESETS,
        clocks.system_clock.freq(),
    );

    let async_motor_i2c = AsyncI2c::new(motor_i2c, 10);
    let motor_i2c_arbiter = ctx.local.i2c_motor_bus.write(Arbiter::new(async_motor_i2c));

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

    let ina260_1 = AsyncINA260::new(ArbiterDevice::new(motor_i2c_arbiter), 0x40, Mono);
    let ina260_2 = AsyncINA260::new(ArbiterDevice::new(motor_i2c_arbiter), 0x41, Mono);
    let ina260_3 = AsyncINA260::new(ArbiterDevice::new(motor_i2c_arbiter), 0x44, Mono);
    let ina260_4 = AsyncINA260::new(ArbiterDevice::new(motor_i2c_arbiter), 0x45, Mono);

    // let mut adc = rp235x_hal::Adc::new(ctx.device.ADC, &mut ctx.device.RESETS);
    // let mut adc_photoresistors: rp235x_hal::adc::AdcPin<Pin<rp235x_hal::gpio::bank0::Gpio40, rp235x_hal::gpio::FunctionNull, rp235x_hal::gpio::PullDown>> = rp235x_hal::adc::AdcPin::new(pins.gpio40).unwrap();

    
    *ctx.local.adc = Some(rp235x_hal::Adc::new(ctx.device.ADC, &mut ctx.device.RESETS));
    let adc = ctx.local.adc.as_mut().unwrap();

    let mut adc_pin_0 = rp235x_hal::adc::AdcPin::new(pins.gpio28.into_floating_input()).unwrap();
   
    let mut adc_fifo = adc
        .build_fifo()
        // Set clock divider to target a sample rate of 1000 samples per second (1ksps).
        // The value was calculated by `(48MHz / 1ksps) - 1 = 47999.0`.
        // Please check the `clock_divider` method documentation for details.
        .clock_divider(47999, 0)
        .set_channel(&mut adc_pin_0)
        //.enable_interrupt(1)
        .start();

    let adc_fifo = Some(adc_fifo);

    let data = ComputeTXBuffer::new();
    let metrics_buf = ComputeRXBuffer::new();
    let rbf = pins.gpio4.into_pull_down_input();



    info!("Peripherals initialized, spawning tasks...");
    heartbeat::spawn().ok();
    // ina_sample::spawn(motor_i2c_arbiter).ok();
    sample_sensors::spawn(avionics_i2c_arbiter).ok();
    info!("Tasks spawned!");
    (
        Shared { data },
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
            adc_fifo_l: adc_fifo,
            adc_outputs: [0u16; 24],
            mp_channel: MpChannel::PD1_4,
            pin19: pins.gpio19.into_pull_type::<PullNone>().into_push_pull_output(),
            pin20: pins.gpio20.into_pull_type::<PullNone>().into_push_pull_output(),
            pin21: pins.gpio21.into_pull_type::<PullNone>().into_push_pull_output(),
            metrics_buf,
            compute_link,

            // adc,
            // adc_photoresistors,
            // mux
        },
    )
}
