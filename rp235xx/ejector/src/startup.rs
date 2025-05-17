use bin_packets::device::PacketDevice;
use common::rbf::{ActiveHighRbf, RbfIndicator};

use defmt::{info, warn};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use fugit::RateExtU32;
use hc12_rs::configuration::baudrates::B9600;
use hc12_rs::configuration::{Channel, HC12Configuration, Power};
use hc12_rs::device::IntoATMode;
use hc12_rs::IntoFU3Mode;
use rp235x_hal::clocks::init_clocks_and_plls;
use rp235x_hal::gpio::PullNone;
use rp235x_hal::pwm::Slices;
use rp235x_hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use rp235x_hal::{Clock, Sio, Watchdog};
use rtic_monotonics::Monotonic;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::{StringDescriptors, UsbDeviceBuilder, UsbVidPid};
use usbd_serial::SerialPort;

use crate::actuators::servo::{EjectionServoMosfet, EjectorServo, Servo};
use crate::device_constants::packets::{JupiterInterface, RadioInterface};
use crate::device_constants::pins::RadioProgrammingPin;
use crate::device_constants::{EjectionDetectionPin, EjectorHC12, EjectorRbf, JupiterUart, RadioUart};
use crate::hal;
use crate::phases::EjectorStateMachine;
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

    // Configure GPIO25 as an output
    let mut led_pin = bank0_pins
        .gpio25
        .into_pull_type::<PullNone>()
        .into_push_pull_output();
    led_pin.set_low().unwrap();

    // Configure GPIOX as a cam output // Change later
    let mut cam_pin = bank0_pins
        .gpio14
        .into_pull_type::<PullNone>()
        .into_push_pull_output();
    cam_pin.set_high().unwrap();

    let mut cam_led_pin = bank0_pins
        .gpio13
        .into_pull_type::<PullNone>()
        .into_push_pull_output();
    cam_led_pin.set_high().unwrap();

    // Looking at the docs originally an LED wasn't reserved for RBF but this could be a thing
    // Also there's some issue with specifically the GPIO that would be used for power or RBF
    // So leaving this commented out for now
    let mut rbf_led_pin = bank0_pins
        .gpio15
        .into_pull_type::<PullNone>()
        .into_push_pull_output();
    rbf_led_pin.set_low().unwrap();

    // Ejector rbf should be pull down - it is high when the rbf is inserted
    let mut rbf: EjectorRbf = ActiveHighRbf::new(bank0_pins.gpio2.into_pull_down_input());

    info!("RBF Pin state: {}", rbf.is_inserted());

    if rbf.inhibited_at_init() {
        rbf_led_pin.set_high().unwrap();
        info!("RBF inhibited at init!");
    }

    // Get clock frequency
    let clock_freq = clocks.peripheral_clock.freq();

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

    // Packet interface to relay packets down
    let jupiter_downlink: JupiterInterface = PacketDevice::new(jupiter_uart);

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
    let hc_programming_pin: RadioProgrammingPin = bank0_pins.gpio20.into_push_pull_output();
    let builder = hc12_rs::device::HC12Builder::<(), (), (), ()>::empty()
        .uart(radio_uart, B9600)
        .programming_resources(hc_programming_pin, timer)
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
    timer_two.delay_ms(300);
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
    timer_two.delay_ms(300);
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
    timer_two.delay_ms(300);
    let hc = match radio.set_power(Power::P8) {
        Ok(link) => {
            info!("HC12 power set to P8");
            link
        }
        Err(e) => {
            warn!("Failed to set HC12 power: {:?}", e.error);
            e.hc12
        }
    };
    let hc: EjectorHC12 = hc.into_fu3_mode().unwrap(); // Infallible
    let radio: RadioInterface = PacketDevice::new(hc);

    // Servo
    let pwm_slices = Slices::new(ctx.device.PWM, &mut ctx.device.RESETS);
    let mut ejection_pwm = pwm_slices.pwm0;
    ejection_pwm.enable();
    ejection_pwm.set_div_int(48);
    // Pin for servo mosfet digital
    let mut mosfet_pin: EjectionServoMosfet = bank0_pins.gpio1.into_push_pull_output();
    mosfet_pin.set_low().unwrap();
    let mut channel_a = ejection_pwm.channel_a;
    let channel_pin = channel_a.output_to(bank0_pins.gpio0);
    channel_a.set_enabled(true);
    let ejection_servo = Servo::new(channel_a, channel_pin, mosfet_pin);
    // Create ejector servo
    let mut ejector_servo: EjectorServo = EjectorServo::new(ejection_servo);
    ejector_servo.enable();
    ejector_servo.hold();

    let gpio_detect: EjectionDetectionPin = bank0_pins.gpio21.reconfigure();

    // Set up USB Device allocator
    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        ctx.device.USB,
        ctx.device.USB_DPRAM,
        clocks.usb_clock,
        true,
        &mut ctx.device.RESETS,
    ));
    unsafe {
        USB_BUS = Some(usb_bus);
    }
    #[allow(static_mut_refs)]
    let usb_bus_ref = unsafe { USB_BUS.as_ref().unwrap() };

    let serial = SerialPort::new(usb_bus_ref);
    let usb_dev = UsbDeviceBuilder::new(usb_bus_ref, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::default()
            .manufacturer("UAH TERMINUS PROGRAM")
            .product("Canonical Toolchain USB Serial Port")
            .serial_number("TEST")])
        .unwrap()
        .device_class(2)
        .build();

    info!("Peripherals initialized, spawning tasks");

    // Tasks
    heartbeat::spawn().ok();
    ejector_sequencer::spawn().ok();
    radio_read::spawn().ok();
    start_cameras::spawn().ok();
    rbf_monitor::spawn().ok();

    (
        Shared {
            usb_device: usb_dev,
            usb_serial: serial,
            clock_freq_hz: clock_freq.to_Hz(),
            state_machine: EjectorStateMachine::new(),
            blink_status_delay_millis: 1000,
            ejector_time_millis: 0,
            suspend_packet_handler: false,
            radio,
            rbf,
            downlink: jupiter_downlink,
            led: led_pin,
        },
        Local {
            ejector_servo,
            ejection_pin: gpio_detect,
            cams: cam_pin,
            cams_led: cam_led_pin,
            rbf_led: rbf_led_pin,
        },
    )
}
