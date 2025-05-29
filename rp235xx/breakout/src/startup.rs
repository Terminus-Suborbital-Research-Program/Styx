use defmt::{error, info, warn};
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
};
use fugit::RateExtU32;
use hc12_rs::IntoFU3Mode;
use rp235x_hal::{
    clocks,
    gpio::{FunctionI2C, FunctionPwm, Pin, PullNone, PullUp},
    pwm::Slices,
    uart::{DataBits, StopBits, UartConfig, UartPeripheral},
    Clock, Sio, Watchdog, I2C,
};
use rtic_sync::arbiter::{i2c::ArbiterDevice, Arbiter};
use tinyframe::writer::BufferingWriter;

use crate::{actuators::servo::Servo, device_constants::DownlinkBuffer};
use crate::{
    app::*,
    device_constants::{
        pins::{AvionicsI2CSclPin, AvionicsI2CSdaPin, EscI2CSclPin, EscI2CSdaPin},
        servos::{
            FlapMosfet, FlapServo, FlapServoPwmPin, FlapServoSlice, RelayMosfet, RelayServo,
            RelayServoPwmPin, RelayServoSlice,
        },
        IcarusRadio,
    },
    peripherals::async_i2c::AsyncI2c,
    Mono,
};

use hc12_rs::{
    configuration::{baudrates::B9600, Channel, HC12Configuration, Power},
    device::IntoATMode,
};

// Sensors
use bme280_rs::AsyncBme280;
use bmi323::AsyncBMI323;
use ina260_terminus::AsyncINA260;
use bmm350::AsyncBMM350;

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
    let mut debug_pin = pins.gpio11.into_push_pull_output();
    debug_pin.set_high().unwrap();
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
                debug_pin.set_high().unwrap();
            } else {
                debug_pin.set_low().unwrap();
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

    info!("UART1 configured, assembling HC-12");
    let builder = hc12_rs::device::HC12Builder::<(), (), (), ()>::empty()
        .uart(uart1_peripheral, B9600)
        .programming_resources(programming, timer)
        .fu3(HC12Configuration::default());

    let radio = match builder.attempt_build() {
        Ok(link) => {
            info!("HC-12 init, link ready");
            link
        }
        Err(e) => {
            panic!("Failed to init HC-12: {}", e.0);
        }
    };
    // Transition to AT mode
    info!("Programming HC12...");
    let radio = radio.into_at_mode().unwrap(); // Infallible
    timer_two.delay_ms(100);
    let radio = match radio.set_baudrate(B9600) {
        Ok(link) => {
            info!("HC12 baudrate set to 9600");
            link
        }
        Err(e) => {
            error!("Failed to set HC12 baudrate: {:?}", e.error);
            e.hc12
        }
    };

    timer_two.delay_ms(150);

    let radio = match radio.set_channel(Channel::Channel1) {
        Ok(link) => {
            info!("HC12 channel set to 1");
            link
        }
        Err(e) => {
            error!("Failed to set HC12 channel: {:?}", e.error);
            e.hc12
        }
    };
    timer_two.delay_ms(150);
    let radio = match radio.set_power(Power::P8) {
        Ok(link) => {
            info!("HC12 power set to P8");
            link
        }
        Err(e) => {
            error!("Failed to set HC12 power: {:?}", e.error);
            e.hc12
        }
    };
    timer_two.delay_ms(150);
    let hc = radio.into_fu3_mode().unwrap(); // Infallible

    let interface: IcarusRadio = BufferingWriter::new(hc);

    // Servo mosfets
    let mut flap_mosfet: FlapMosfet = pins.gpio2.into_function();
    flap_mosfet.set_low().unwrap();

    let mut relay_mosfet: RelayMosfet = pins.gpio0.into_function();
    relay_mosfet.set_low().unwrap();

    // Servo PWMs
    let slices = Slices::new(ctx.device.PWM, &mut ctx.device.RESETS);

    let mut flap_slice: FlapServoSlice = slices.pwm1;
    flap_slice.set_div_int(64);
    flap_slice.set_ph_correct();
    flap_slice.enable();

    let mut relay_slice: RelayServoSlice = slices.pwm0;
    relay_slice.set_div_int(64);
    relay_slice.set_ph_correct();
    relay_slice.enable();

    // Flap servo
    let mut flap_channel = flap_slice.channel_b;
    flap_channel.set_enabled(true);
    let flap_pin: FlapServoPwmPin =
        flap_channel.output_to(pins.gpio3.into_function::<FunctionPwm>());
    let flap_servo: FlapServo = Servo::new(flap_channel, flap_pin, flap_mosfet);

    // Relay servo
    let mut relay_channel = relay_slice.channel_b;
    relay_channel.set_enabled(true);
    let relay_pin: RelayServoPwmPin =
        relay_channel.output_to(pins.gpio1.into_function::<FunctionPwm>());
    let relay_servo: RelayServo = Servo::new(relay_channel, relay_pin, relay_mosfet);

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
    let bmm350 = AsyncBMM350::new(ArbiterDevice::new(avionics_i2c_arbiter), 0x14, Mono);
    let bmi323 = AsyncBMI323::new(ArbiterDevice::new(avionics_i2c_arbiter), 0x69, Mono);
    let bme280 = AsyncBme280::new(ArbiterDevice::new(avionics_i2c_arbiter), Mono);

    let ina260_1 = AsyncINA260::new(ArbiterDevice::new(motor_i2c_arbiter), 0x40, Mono);
    let ina260_2 = AsyncINA260::new(ArbiterDevice::new(motor_i2c_arbiter), 0x41, Mono);
    let ina260_3 = AsyncINA260::new(ArbiterDevice::new(motor_i2c_arbiter), 0x42, Mono);

    let data = DownlinkBuffer::new();

    // let mut rbf = pins.gpio4.into_pull_down_input();

    // Wait for the "Remove Before Flight" (RBF) pin to go low.
    // The RBF pin is a safety mechanism that ensures certain tasks
    // do not start until the pin is removed. This loop continuously
    // checks the state of the pin and delays task initialization
    // until the pin is confirmed to be low.
    // let mut rbf_high = true;
    // while rbf_high {
    //     if rbf.is_low().unwrap() {
    //         rbf_high = false;
    //         info!("RBF is low.");
    //     } else {
    //         rbf_high = true;
    //         info!("RBF is high.");
    //     }
    // }

    info!("Peripherals initialized, spawning tasks...");
    // heartbeat::spawn().ok();
    // mode_sequencer::spawn().ok();
    ina_sample::spawn(motor_i2c_arbiter).ok();
    // sample_sensors::spawn(avionics_i2c_arbiter).ok();
    // inertial_nav::spawn().ok();
    // radio_send::spawn().ok();
    info!("Tasks spawned!");
    (
        Shared { data },
        Local {
            radio: interface,
            flap_servo,
            relay_servo,
            led: led_pin,
            bmm350,
            bmi323,
            bme280,
            ina260_1,
            ina260_2,
            ina260_3,
        },
    )
}
