use bin_packets::phases::IcarusPhase;
use defmt::info;
use defmt::warn;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use fugit::RateExtU32;
use rp235x_hal::clocks;
use rp235x_hal::gpio::FunctionI2C;
use rp235x_hal::gpio::FunctionPwm;
use rp235x_hal::gpio::Pin;
use rp235x_hal::gpio::PullNone;
use rp235x_hal::gpio::PullUp;
use rp235x_hal::pwm::Slices;
use rp235x_hal::uart::DataBits;
use rp235x_hal::uart::StopBits;
use rp235x_hal::uart::UartConfig;
use rp235x_hal::uart::UartPeripheral;
use rp235x_hal::Clock;
use rp235x_hal::Sio;
use rp235x_hal::Watchdog;
use rp235x_hal::I2C;
use rtic_sync::arbiter::i2c::ArbiterDevice;
use rtic_sync::arbiter::Arbiter;
use rtic_sync::signal::Signal;
// use usb_device::bus::UsbBusAllocator;
// use usbd_serial::SerialPort;

use crate::actuators::servo::Servo;

use crate::app::*;
use crate::device_constants::pins::{AvionicsI2CSclPin, AvionicsI2CSdaPin};
use crate::device_constants::pins::{EscI2CSclPin, EscI2CSdaPin};
use crate::device_constants::servos::FlapMosfet;
use crate::device_constants::servos::FlapServo;
use crate::device_constants::servos::FlapServoPwmPin;
use crate::device_constants::servos::FlapServoSlice;
use crate::device_constants::servos::RelayMosfet;
use crate::device_constants::servos::RelayServo;
use crate::device_constants::servos::RelayServoPwmPin;
use crate::device_constants::servos::RelayServoSlice;
use crate::device_constants::servos::FLAP_SERVO_LOCKED;
use crate::device_constants::servos::RELAY_SERVO_LOCKED;
use crate::device_constants::INAData;
use crate::device_constants::IcarusRadio;
use crate::device_constants::IcarusStateMachine;
use crate::hal;
use crate::peripherals::async_i2c::AsyncI2c;
use crate::phases::StateMachine;
use crate::phases::StateMachineListener;
use crate::Mono;

use hc12_rs::configuration::baudrates::B9600;
use hc12_rs::configuration::{Channel, HC12Configuration, Power};
use hc12_rs::device::IntoATMode;
use hc12_rs::IntoFU3Mode;

// Sensors
use bme280_rs::AsyncBme280;
use ina260_terminus::AsyncINA260;

// Logs our time for demft
defmt::timestamp!("{=u64:us}", { epoch_ns() });

pub fn startup(mut ctx: init::Context) -> (Shared, Local) {
    // Reset the spinlocks - this is skipped by soft-reset
    unsafe {
        hal::sio::spinlock_reset();
    }

    // Set up clocks
    let mut watchdog = Watchdog::new(ctx.device.WATCHDOG);

    info!("Good morning sunshine! Icarus is awake!");

    Mono::start(ctx.device.TIMER0, &ctx.device.RESETS);

    // The single-cycle I/O block controls our GPIO pins
    let sio = Sio::new(ctx.device.SIO);

    // Set the pins to their default state
    let pins = hal::gpio::Pins::new(
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
            loop {}
            panic!("Failed to init clocks");
        }
    };

    // Configure GPIO25 as an output
    let mut led_pin = pins
        .gpio25
        .into_pull_type::<PullNone>()
        .into_push_pull_output();
    led_pin.set_low().unwrap();

    // Get clock frequency
    let clock_freq = clocks.peripheral_clock.freq();

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

    let programming = pins.gpio10.into_push_pull_output();
    // Copy the timer
    let timer = hal::Timer::new_timer1(ctx.device.TIMER1, &mut ctx.device.RESETS, &clocks);
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
            warn!("Failed to set HC12 baudrate: {:?}", e.error);
            e.hc12
        }
    };

    let radio = match radio.set_channel(Channel::Channel1) {
        Ok(link) => {
            info!("HC12 channel set to 1");
            link
        }
        Err(e) => {
            warn!("Failed to set HC12 channel: {:?}", e.error);
            e.hc12
        }
    };
    let radio = match radio.set_power(Power::P8) {
        Ok(link) => {
            info!("HC12 power set to P8");
            link
        }
        Err(e) => {
            warn!("Failed to set HC12 power: {:?}", e.error);
            e.hc12
        }
    };
    let hc = radio.into_fu3_mode().unwrap(); // Infallible

    let interface: IcarusRadio = bin_packets::device::PacketDevice::new(hc);

    // Servo mosfets
    let mut flap_mosfet: FlapMosfet = pins.gpio3.into_function();
    let mut relay_mosfet: RelayMosfet = pins.gpio4.into_function();
    flap_mosfet.set_low().unwrap();
    relay_mosfet.set_low().unwrap();

    // Servo PWMs
    let slices = Slices::new(ctx.device.PWM, &mut ctx.device.RESETS);
    let flap_slice: FlapServoSlice = slices.pwm0;
    let relay_slice: RelayServoSlice = slices.pwm1;

    // Flap servo
    let mut flap_channel = flap_slice.channel_b;
    flap_channel.set_enabled(true);
    let flap_pin: FlapServoPwmPin =
        flap_channel.output_to(pins.gpio1.into_function::<FunctionPwm>());
    let mut flap_servo: FlapServo = Servo::new(flap_channel, flap_pin, flap_mosfet);

    // Relay servo
    let mut relay_channel = relay_slice.channel_a;
    relay_channel.set_enabled(true);
    let relay_pin: RelayServoPwmPin =
        relay_channel.output_to(pins.gpio2.into_function::<FunctionPwm>());
    let mut relay_servo: RelayServo = Servo::new(relay_channel, relay_pin, relay_mosfet);

    // Lock initially
    flap_servo.set_angle(FLAP_SERVO_LOCKED);
    relay_servo.set_angle(RELAY_SERVO_LOCKED);
    flap_servo.enable();
    relay_servo.enable();

    // Sensors
    // Init I2C pins
    let motor_sda_pin: Pin<EscI2CSdaPin, FunctionI2C, PullUp> = pins.gpio18.reconfigure();
    let motor_scl_pin: Pin<EscI2CSclPin, FunctionI2C, PullUp> = pins.gpio19.reconfigure();

    let motor_i2c = I2C::new_controller(
        ctx.device.I2C1,
        motor_sda_pin,
        motor_scl_pin,
        RateExtU32::kHz(400),
        &mut ctx.device.RESETS,
        clocks.system_clock.freq(),
    );
    let async_motor_i2c = AsyncI2c::new(motor_i2c, 10);
    let motor_i2c_arbiter = ctx.local.i2c_motor_bus.write(Arbiter::new(async_motor_i2c));

    let avionics_sda_pin: Pin<AvionicsI2CSdaPin, FunctionI2C, PullUp> = pins.gpio16.reconfigure();
    let avionics_scl_pin: Pin<AvionicsI2CSclPin, FunctionI2C, PullUp> = pins.gpio17.reconfigure();

    let avionics_i2c = I2C::new_controller(
        ctx.device.I2C0,
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
    let bme280 =
        AsyncBme280::new_with_address(ArbiterDevice::new(avionics_i2c_arbiter), 0x77, Mono);

    // State machine
    let mut state_machine: IcarusStateMachine = StateMachine::new();
    let esc_state_signal: Signal<IcarusPhase> = Signal::new();
    ctx.local.esc_state_signal.write(esc_state_signal);

    let r = unsafe { ctx.local.esc_state_signal.assume_init_ref() };
    let (writer, reader) = r.split();
    state_machine.add_channel(writer).ok();
    let esc_listener = StateMachineListener::new(reader);

    let mut ina260_1 = AsyncINA260::new(ArbiterDevice::new(motor_i2c_arbiter), 0x40, Mono);
    let mut ina260_2 = AsyncINA260::new(ArbiterDevice::new(motor_i2c_arbiter), 0x41, Mono);
    let mut ina260_3 = AsyncINA260::new(ArbiterDevice::new(motor_i2c_arbiter), 0x42, Mono);

    let ina_data = INAData::default();

    info!("Peripherals initialized, spawning tasks...");
    // heartbeat::spawn().ok();
    // radio_flush::spawn().ok();
    motor_drivers::spawn(motor_i2c_arbiter, esc_listener).ok();
    sample_sensors::spawn(avionics_i2c_arbiter).ok();
    inertial_nav::spawn().ok();
    radio_send::spawn().ok();
    info!("Tasks spawned!");
    (
        Shared {
            ina_data,
            flap_servo,
            relay_servo,
            clock_freq_hz: clock_freq.to_Hz(),
            state_machine,
            radio: interface,
        },
        Local {
            led: led_pin,
            bme280,
            ina260_1,
            ina260_2,
            ina260_3,
        },
    )
}
