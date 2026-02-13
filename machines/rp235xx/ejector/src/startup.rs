#![warn(missing_docs)]

#![warn(missing_docs)]

use defmt::{info, warn};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use fugit::RateExtU32;
use heapless::Deque;
use rp235x_hal::adc::AdcPin;
use rp235x_hal::clocks::init_clocks_and_plls;
use rp235x_hal::gpio::{FunctionSio, FunctionSio, PinState, PullNone, FunctionUart, SioInput, SioInput};
use rp235x_hal::pwm::Slices;
use rp235x_hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use rp235x_hal::{Clock, Sio, Watchdog};
use rtic_monotonics::Monotonic;

use mcp9600::{
    ADCResolution, BurstModeSamples, ColdJunctionResolution, DeviceAddr, 
    FilterCoefficient, MCP9600, ShutdownMode, ThermocoupleType
};
use rp235x_hal::i2c::I2C;
// use rp235x_hal::timer::monotonic::Monotonic;

use crate::actuators::electromag::{ElectroMagnet, ElectroMagnetPolarity, HBridge};
use crate::actuators::servo::{EjectionServoMosfet, EjectorServo, Servo};
use crate::device_constants::pins::{CamMosfetPin};
use crate::device_constants::{
    EjectionDetectionPin, GreenLed, JupiterUart, RedLed, SAMPLE_COUNT, ThermoI2cBus,
};
use crate::hal;
use crate::{app::*, Mono};

// Timestamp for logging
defmt::timestamp!("{=u64:us}", {
    Mono::now().duration_since_epoch().to_nanos()
});

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

    let mut thermocouple = MCP9600::new(thermo_i2c_bus, DeviceAddr::AD7)
        .expect("Failed to initialize MCP9600");

    thermocouple.set_sensor_configuration(
        ThermocoupleType::TypeK,
        FilterCoefficient::FilterMedium,
    ).unwrap();

    thermocouple.set_device_configuration(
        ColdJunctionResolution::High,
        ADCResolution::Bit18,
        BurstModeSamples::Sample1,
        ShutdownMode::NormalMode,
    ).unwrap();

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

    // Pin setup for UART1
    let uart1_pins = (
        bank0_pins.gpio8.into_function(),
        bank0_pins.gpio9.into_function(),
    );
    let mut radio_uart: RadioUart =
        UartPeripheral::new(ctx.device.UART1, uart1_pins, &mut ctx.device.RESETS)
            .enable(
                UartConfig::new(9600_u32.Hz(), DataBits::Eight, None, StopBits::One),
                clocks.peripheral_clock.freq(),
            )
            .unwrap();
    radio_uart.enable_rx_interrupt(); // Make sure we can drive our interrupts
                                      //let hc_programming_pin: RadioProgrammingPin = bank0_pins.gpio20.into_push_pull_output();
                                      //let builder = hc12_rs::device::HC12Builder::<(), (), (), ()>::empty()
                                      //    .uart(radio_uart, B9600)
                                      //    .programming_resources(hc_programming_pin, timer)
                                      //    .fu3(HC12Configuration::default());

    //let radio = match builder.attempt_build() {
    //    Ok(link) => {
    //        info!("HC-12 init, link ready");
    //        link
    //    }
    //    Err(e) => {
    //        panic!("Failed to init HC-12: {}", e.0);
    //    }
    //};

    // Transition to AT mode
    //info!("Programming HC12...");
    //let radio = radio.into_at_mode().unwrap(); // Infallible
    timer_two.delay_ms(300);
    //let radio = match radio.set_baudrate(B9600) {
    //    Ok(link) => {
    //        info!("HC12 baudrate set to 9600");
    //        link
    //    }
    //    Err(e) => {
    //        warn!("Failed to set HC12 baudrate: {:?}", e.error);
    //        e.hc12
    //    }
    //};
    timer_two.delay_ms(300);
    //let radio = match radio.set_channel(Channel::Channel1) {
    //    Ok(link) => {
    //        info!("HC12 channel set to 1");
    //        link
    //    }
    //    Err(e) => {
    //        warn!("Failed to set HC12 channel: {:?}", e.error);
    //        e.hc12
    //    }
    //};
    timer_two.delay_ms(300);
    // let hc = match radio.set_power(Power::P8) {
    //     Ok(link) => {
    //         info!("HC12 power set to P8");
    //         link
    //     }
    //     Err(e) => {
    //         warn!("Failed to set HC12 power: {:?}", e.error);
    //         e.hc12
    //     }
    // };
    // let hc: EjectorHC12 = hc.into_fu3_mode().unwrap(); // Infallible

    // Servo
    let pwm_slices = Slices::new(ctx.device.PWM, &mut ctx.device.RESETS);
    let mut ejection_servo_pwm = pwm_slices.pwm0;
    ejection_servo_pwm.enable();
    ejection_servo_pwm.set_div_int(48);

    let mut ejection_emag_pwm = pwm_slices.pwm2;
    ejection_emag_pwm.enable();
    ejection_emag_pwm.set_div_int(48);

    // Pin for servo mosfet digital
    let mut mosfet_pin: EjectionServoMosfet = bank0_pins.gpio1.into_push_pull_output();
    mosfet_pin.set_low().unwrap();
    let mut channel_a = ejection_servo_pwm.channel_a;
    let channel_pin = channel_a.output_to(bank0_pins.gpio0);
    channel_a.set_enabled(true);
    let ejection_servo = Servo::new(channel_a, channel_pin, mosfet_pin);

    // Add emag variable
    let mut echannel_a = ejection_emag_pwm.channel_a;
    let mut echannel_b = ejection_emag_pwm.channel_b;

    //let mut rt = bank0_pins.gpio20.into_push_pull_output();

    let emag_pwm_pin1 = echannel_b.output_to(bank0_pins.gpio21);
    let emag_pwm_pin2 = echannel_a.output_to(bank0_pins.gpio20);
    let emag_arming_pin = bank0_pins.gpio22.into_push_pull_output();

    //let emag_channels = (echannel_a, echannel_b);

    let mut ejector_magnet = ElectroMagnet::new(
        HBridge::new(echannel_a, echannel_b, emag_arming_pin),
        ElectroMagnetPolarity::State1,
    );
    // Create ejector servo
    let mut ejector_servo: EjectorServo = EjectorServo::new(ejection_servo);
    ejector_servo.enable();
    ejector_servo.hold();

    let gpio_detect: EjectionDetectionPin = bank0_pins.gpio24.into_pull_down_input();

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

    // Tasks
    heartbeat::spawn().ok();
    ejector_sequencer::spawn().ok();
    camera_sequencer::spawn().ok();
    poll_temperature::spawn().ok();
    downlink_jupiter::spawn().ok();

    (
        Shared {
            downlink_packets: Deque::new(),
            samples_buffer: [0u16; SAMPLE_COUNT],
        },
        Local {
            camera_mosfet: cam_pin,
            onboard_led: led_pin,
            downlink: jupiter_uart,
            ejector_servo,
            ejection_pin: gpio_detect,
            ejecctor_magnet: ejector_magnet,
            arming_led: red_led_pin,
            packet_led: packet_indicator,
            thermocouple,
        },
    )
}
