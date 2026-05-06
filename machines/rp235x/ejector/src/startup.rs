//! Startup initialization for the Ejector

#![warn(missing_docs, clippy::unwrap_used)]

use common_states::rbf;
use defmt::{info, warn};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use embedded_hal_bus::spi::ExclusiveDevice;
use fugit::RateExtU32;
use heapless::Deque;
use mcp9600::{
    ADCResolution, BurstModeSamples, ColdJunctionResolution, DeviceAddr, FilterCoefficient,
    ShutdownMode, ThermocoupleType, MCP9600,
};
use rp235x_hal::adc::AdcPin;
use rp235x_hal::clocks::init_clocks_and_plls;
use rp235x_hal::gpio::{
    FunctionSio, FunctionSpi, FunctionUart, PinState, PullDown, PullNone, SioInput, FunctionPio0
};
use rp235x_hal::i2c::I2C;
use rp235x_hal::pwm::Slices;
use rp235x_hal::spi::Spi;
use rp235x_hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use rp235x_hal::{Clock, Sio, Watchdog};
use rtic_monotonics::Monotonic;
use rtic_sync::make_signal;
use rtic_sync::signal::{self, Signal};
use rp235x_hal::pio::PIOExt;
use ws2812_pio::Ws2812Direct;
use smart_leds::{SmartLedsWrite, RGB8};

// use rp235x_hal::timer::monotonic::Monotonic;

use crate::actuators::electromag::{ElectroMagnet, ElectroMagnetPolarity, HBridge};
use crate::actuators::servo::{EjectionServoMosfet, EjectorServo, Servo};
use crate::device_constants::pins::{CamMosfetPin, RBFPin};
use crate::device_constants::{
    Cam1, Cam1Pin, Cam2, EjectionDetectionPin, JupiterUart, RGBLed, RGBStatus, ThermoI2CSclPin,
    ThermoI2CSdaPin, ThermoI2cBus, SAMPLE_COUNT,
};
use crate::{app::*, Mono};
use crate::{hal, sd_card};

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

    let timer = hal::Timer::new_timer1(ctx.device.TIMER1, &mut ctx.device.RESETS, &clocks);

    // Debugging on-board LED pin
    // let mut led_pin = bank0_pins
    //     .gpio25
    //     .into_pull_type::<PullNone>()
    //     .into_push_pull_output();
    // led_pin.set_low().unwrap();

    // Red led pin - gets set high when armed
    let cam1: Cam1 = bank0_pins
        .gpio10
        .into_push_pull_output_in_state(PinState::High)
        .reconfigure();

    // Control pins for camera - pull high to turn on cameras
    let cam_pin: CamMosfetPin = bank0_pins
        .gpio12
        .into_push_pull_output_in_state(PinState::Low);

    // Frame received indicator
    let cam2: Cam2 = bank0_pins
        .gpio11
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
        MCP9600::new(thermo_i2c_bus, DeviceAddr::AD7).expect("Failed to initialize MCP9600 struct");

    if let Err(_) = thermocouple.set_sensor_configuration(ThermocoupleType::TypeK, FilterCoefficient::FilterMedium) {
        warn!("MCP9600 missing or failed to set sensor config");
    }

    if let Err(_) = thermocouple.set_device_configuration(
        ColdJunctionResolution::High,
        ADCResolution::Bit18,
        BurstModeSamples::Sample1,
        ShutdownMode::NormalMode,
    ) {
        warn!("MCP9600 missing or failed to set device config");
    }

    // adc.free_running(&gegier_pin);
    // loop {
    //     adc.wait_ready();
    //     let reading = adc.read_single();
    //     if reading > 100 {
    //         info!("Reading: {}", reading as f32 * 3.3 / 4096.0);
    //     }
    // }

    let spi_mosi = bank0_pins.gpio19.into_function::<FunctionSpi>();
    let spi_miso = bank0_pins.gpio16.into_function::<FunctionSpi>();
    let spi_sck = bank0_pins.gpio18.into_function::<FunctionSpi>();
    let spi_cs = bank0_pins
        .gpio17
        .into_push_pull_output_in_state(PinState::High);

    let spi_bus =
        rp235x_hal::spi::Spi::<_, _, _, 8>::new(ctx.device.SPI0, (spi_mosi, spi_miso, spi_sck));

    let spi = spi_bus.init(
        &mut ctx.device.RESETS,
        clocks.peripheral_clock.freq(),
        400.kHz(), // card initialization happens at low baud rate
        embedded_hal::spi::MODE_0,
    );

    let spi = ExclusiveDevice::new(spi, spi_cs, timer.clone()).unwrap();

    let sd_card = sd_card::EjectorSdCard::new(spi, timer.clone());

    let mut timer_two = timer;

    // Jupiter downlink UART
    let jupiter_uart: JupiterUart = UartPeripheral::new(
        ctx.device.UART0,
        (
            bank0_pins.gpio0.into_function(),
            bank0_pins.gpio1.into_function(),
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
    let pwm_slices = Slices::new(ctx.device.PWM, &mut ctx.device.RESETS);

    // Servo
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

    // Functionality currently not enabled
    // let gpio_detect: EjectionDetectionPin = bank0_pins.gpio8.into_pull_down_input();

    

    // let rgb_ctl_pin: RGBLed = bank0_pins
    //     .gpio24
    //     // .gpio26
    //     .into_pull_type::<PullNone>()
    //     .into_push_pull_output();

    let mut rgb_wake = bank0_pins.gpio25.into_push_pull_output();

    rgb_wake.set_high().unwrap();

    let (mut pio, sm0, _, _, _) = ctx.device.PIO0.split(&mut ctx.device.RESETS);

    let rgb_ctl_pin = bank0_pins
        .gpio24
        .into_pull_type::<PullDown>()
        .into_function::<FunctionPio0>();

    let mut rgb_driver = Ws2812Direct::new(
        rgb_ctl_pin,
        &mut pio,
        sm0,
        clocks.peripheral_clock.freq(),
    );


    // let sys_freq = clocks.system_clock.freq().to_Hz();
    // let mut rgb_driver = WS2812::new(rgb_ctl_pin, (sys_freq / 2) as u64);
   
    // let dim_red     = Color([50, 0, 0]);
    // let dim_green   = Color([0, 50, 0]);
    // let dim_blue    = Color([0, 0, 50]);

    // let dim_yellow  = Color([40, 40, 0]);
    // let dim_cyan    = Color([0, 40, 40]);
    // let dim_magenta = Color([40, 0, 40]);

    // let dim_orange  = Color([50, 20, 0]);
    // let dim_purple  = Color([25, 0, 50]);
    // let dim_white   = Color([30, 30, 30]);
    // let off         = Color([0, 0, 0]);

    let dim_red     = RGB8::new(0, 0, 0);
    let dim_green   = RGB8::new(0, 0, 0);
    let dim_blue    = RGB8::new(0, 0, 0);

    let dim_yellow  = RGB8::new(0, 0, 0);
    let dim_cyan    = RGB8::new(0, 0, 0);
    let dim_magenta = RGB8::new(0, 0, 0);

    let dim_orange  = RGB8::new(0, 0, 0);
    let dim_purple  = RGB8::new(0, 0, 0);
    let dim_white   =RGB8::new(0, 0, 0);
    let off         = RGB8::new(0, 0, 0);


    let current_colors = [
        dim_red,
        dim_green,
        dim_blue,

        dim_yellow,
        dim_cyan,
        dim_magenta,

        dim_orange,
        dim_purple,
        dim_white,
        off,
        off,
        off,
    ];

    rgb_driver.write(current_colors.iter().cloned()).unwrap();
        

           

    
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

    let (ejection_trigger_tx, ejection_trigger_rx) = make_signal!(());

    // Tasks

    poll_rbf::spawn().ok();
    heartbeat::spawn().ok();
    ejector_sequencer::spawn().ok();
    // camera_sequencer::spawn().ok();
    poll_temperature::spawn().ok();
    downlink_jupiter::spawn().ok();
    // write_sd_card::spawn().ok();
    // rgb_driver.send_color([Color::red()]);
    rx_from_jupiter::spawn().ok();
    set_rgb_status::spawn().ok();

    (
        Shared {
            downlink_packets: Deque::new(),
            samples_buffer: [0u16; SAMPLE_COUNT],
            ejection_enabled: false,
            sd_card: sd_card,
            status_config,
            temp_store: Deque::new(),
        },
        Local {
            camera_mosfet: cam_pin,
            // onboard_led: led_pin,
            status_link,
            downlink,
            ejector_servo,
            rbf_pin: rbf_pin,
            ejecctor_magnet: ejector_magnet,
            // arming_led: red_led_pin,
            // packet_led: packet_indicator,
            thermocouple,
            rgb_driver,
            ejection_trigger_tx,
            ejection_trigger_rx,
        },
    )
}
