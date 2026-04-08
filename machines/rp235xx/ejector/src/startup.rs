//! Startup initialization for the Ejector

#![warn(missing_docs)]

use common_states::rbf;
use defmt::{info, warn};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use fugit::RateExtU32;
use heapless::Deque;
use rp235x_hal::adc::AdcPin;
use rp235x_hal::clocks::init_clocks_and_plls;
use rp235x_hal::gpio::{FunctionSio, FunctionUart, PinState, PullNone, SioInput};
use rp235x_hal::pwm::Slices;
use rp235x_hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use rp235x_hal::{Clock, Sio, Watchdog};
use rtic_monotonics::Monotonic;

use mcp9600::{
    ADCResolution, BurstModeSamples, ColdJunctionResolution, DeviceAddr, FilterCoefficient,
    ShutdownMode, ThermocoupleType, MCP9600,
};
use ws2812_rs::WS2812;
use rp235x_hal::i2c::I2C;
// use rp235x_hal::timer::monotonic::Monotonic;

use crate::actuators::electromag::{ElectroMagnet, ElectroMagnetPolarity, HBridge};
use crate::actuators::servo::{EjectionServoMosfet, EjectorServo, Servo};
use crate::device_constants::pins::{CamMosfetPin, RBFPin};
use crate::device_constants::{
    EjectionDetectionPin, GreenLed, JupiterUart, RedLed, ThermoI2cBus, SAMPLE_COUNT, RGBLed,
    RGBStatus,
};
use crate::hal;
use crate::{app::*, Mono};


// Timestamp for logging
defmt::timestamp!("{=u64:us}", {
    Mono::now().duration_since_epoch().to_nanos()
});

/// Initialization
pub fn startup(mut ctx: init::Context<'_>) -> (Shared, Local) {
    // Reset the spinlocks - this is skipped by soft-reset
    unsafe {
        hal::sio::spinlock_reset();
    }

    info!("Ejector startup");

    // Set up clocks
    let mut watchdog = Watchdog::new(ctx.device.WATCHDOG);
    let clocks = init_clocks_and_plls(
        XTAL_FREQ_HZ,
        ctx.device.XOSC,
        ctx.device.CLOCKS,
        ctx.device.PLL_SYS,
        ctx.device.PLL_USB,
        &mut ctx.device.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    Mono::start(ctx.device.TIMER0, &ctx.device.RESETS);

    // The single-cycle I/O block controls our GPIO pins
    let sio = Sio::new(ctx.device.SIO);

    // Set the pins to their default state
    let bank0_pins = hal::gpio::Pins::new(
        ctx.device.IO_BANK0,
        ctx.device.PADS_BANK0,
        sio.gpio_bank0,
        &mut ctx.device.RESETS,
    );

    // Debugging on-board LED pin
    let mut led_pin = bank0_pins
        .gpio25
        .into_pull_type::<PullNone>()
        .into_push_pull_output();
    led_pin.set_low().unwrap();

    // Red led pin - gets set high when armed
    let red_led_pin: RedLed = bank0_pins
        .gpio11
        .into_push_pull_output_in_state(PinState::High)
        .reconfigure();

    // Control pins for camera - pull high to turn on cameras
    let cam_pin: CamMosfetPin = bank0_pins
        .gpio12
        .into_push_pull_output_in_state(PinState::Low);

    // Frame received indicator
    let packet_indicator: GreenLed = bank0_pins
        .gpio10
        .into_push_pull_output_in_state(PinState::High)
        .reconfigure();

    let sda_pin = bank0_pins.gpio32.into_pull_type().into_function();
    let scl_pin = bank0_pins.gpio33.into_pull_type().into_function();

    let thermo_i2c_bus: ThermoI2cBus = I2C::i2c0(
        ctx.device.I2C0,
        sda_pin,
        scl_pin,
        100_u32.kHz(),
        &mut ctx.device.RESETS,
        clocks.peripheral_clock.freq(),
    );

    let mut thermocouple =
        MCP9600::new(thermo_i2c_bus, DeviceAddr::AD7).expect("Failed to initialize MCP9600");

    thermocouple
        .set_sensor_configuration(ThermocoupleType::TypeK, FilterCoefficient::FilterMedium)
        .unwrap();

    thermocouple
        .set_device_configuration(
            ColdJunctionResolution::High,
            ADCResolution::Bit18,
            BurstModeSamples::Sample1,
            ShutdownMode::NormalMode,
        )
        .unwrap();

    // adc.free_running(&gegier_pin);
    // loop {
    //     adc.wait_ready();
    //     let reading = adc.read_single();
    //     if reading > 100 {
    //         info!("Reading: {}", reading as f32 * 3.3 / 4096.0);
    //     }
    // }

    let timer = hal::Timer::new_timer1(ctx.device.TIMER1, &mut ctx.device.RESETS, &clocks);
    let mut timer_two = timer;

    // Jupiter downlink UART
    let jupiter_uart: JupiterUart = UartPeripheral::new(
        ctx.device.UART0,
        (
            bank0_pins.gpio16.into_function(),
            bank0_pins.gpio17.into_function(),
        ),
        &mut ctx.device.RESETS,
    )
    .enable(
        UartConfig::new(115200_u32.Hz(), DataBits::Eight, None, StopBits::One),
        clocks.peripheral_clock.freq(),
    )
    .unwrap();

    let (status_link, downlink) = jupiter_uart.split();


    // Servo
    let mut pwm_slices = Slices::new(ctx.device.PWM, &mut ctx.device.RESETS);
    let mut power_servo_pwm = pwm_slices.pwm2;
    let mut ejection_servo_pwm = pwm_slices.pwm3;

    ejection_servo_pwm.enable();
    ejection_servo_pwm.set_div_int(48);

    power_servo_pwm.enable();
    power_servo_pwm.set_div_int(48);

    // Pin for servo mosfet digital
    let mut mosfet_pin: EjectionServoMosfet = bank0_pins.gpio6.into_push_pull_output();
    mosfet_pin.set_low().unwrap();
    let mut channel_b = ejection_servo_pwm.channel_b;
    let channel_pin = channel_b.output_to(bank0_pins.gpio7);
    channel_b.set_enabled(true);
    let ejection_servo = Servo::new(channel_b, channel_pin, mosfet_pin);

    let mut power_mosfet_pin = bank0_pins.gpio4.into_push_pull_output();
    power_mosfet_pin.set_low().unwrap();
    let mut channel_a = power_servo_pwm.channel_b;
    let channel_pin = channel_a.output_to(bank0_pins.gpio5);
    channel_a.set_enabled(true);
    let mut power_servo = Servo::new(channel_a, channel_pin, power_mosfet_pin);
    power_servo.enable();
    power_servo.set_angle(90);

    // Add emag variables
    let emag_pin1 = bank0_pins.gpio21.into_push_pull_output();
    let emag_pin2 = bank0_pins.gpio20.into_push_pull_output();
    let emag_arming_pin = bank0_pins.gpio22.into_push_pull_output();

    //let emag_channels = (echannel_a, echannel_b);

    let mut ejector_magnet = ElectroMagnet::new(
        HBridge::new(emag_pin1, emag_pin2, emag_arming_pin),
        ElectroMagnetPolarity::Attract,
    );

    let mut rbf_pin: RBFPin = bank0_pins.gpio2.into_pull_down_input();

    // Create ejector servo
    let mut ejector_servo: EjectorServo = EjectorServo::new(ejection_servo);
    ejector_servo.enable();
    ejector_servo.hold();

    let gpio_detect: EjectionDetectionPin = bank0_pins.gpio24.into_pull_down_input();

    let rgb_ctl_pin: RGBLed = bank0_pins.gpio45
        .into_pull_type::<PullNone>()
        .into_push_pull_output();

    let sys_freq = clocks.system_clock.freq().to_Hz();
    let mut rgb_driver = WS2812::new(rgb_ctl_pin, sys_freq as u64); 

    // SI1445 I2C
    // let guard_i2c: GuardI2C = I2C::i2c1(
    //     ctx.device.I2C1,
    //     bank0_pins.gpio26.reconfigure(),
    //     bank0_pins.gpio27.reconfigure(),
    //     100.kHz(),
    //     &mut ctx.device.RESETS,
    //     12.MHz(),
    // );

    info!("Peripherals initialized, spawning tasks");

    let status_config = RGBStatus::default();

    // Tasks

    poll_rbf::spawn().ok();
    heartbeat::spawn().ok();
    ejector_sequencer::spawn().ok();
    camera_sequencer::spawn().ok();
    poll_temperature::spawn().ok();
    downlink_jupiter::spawn().ok();

    (
        Shared {
            downlink_packets: Deque::new(),
            samples_buffer: [0u16; SAMPLE_COUNT],
            ejection_enabled: false,
            status_config,
        },
        Local {
            camera_mosfet: cam_pin,
            onboard_led: led_pin,
            status_link,
            downlink,
            ejector_servo,
            rbf_pin: rbf_pin,
            ejection_pin: gpio_detect,
            ejecctor_magnet: ejector_magnet,
            arming_led: red_led_pin,
            packet_led: packet_indicator,
            thermocouple,
            rgb_driver,
        },
    )
}
