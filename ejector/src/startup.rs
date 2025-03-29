use defmt::info;
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
use rtic_sync::make_channel;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::{StringDescriptors, UsbDeviceBuilder, UsbVidPid};
use usbd_serial::SerialPort;

use crate::actuators::servo::{EjectionServoMosfet, EjectorServo, Servo};
use crate::communications::serial_handler::{self, HeaplessString, MAX_USB_LINES};
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

    info!("Execution Starting");

    // Channel for sending strings to the USB console
    let (usb_console_line_sender, usb_console_line_receiver) =
        make_channel!(HeaplessString, MAX_USB_LINES);

    // Channel for incoming commands from the USB console
    let (usb_console_command_sender, usb_console_command_receiver) =
        make_channel!(HeaplessString, MAX_USB_LINES);

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
    // Start the heartbeat task
    heartbeat::spawn().ok();

    // Get clock frequency
    let clock_freq = clocks.peripheral_clock.freq();

    // Pin setup for UART1
    let uart1_pins = (
        bank0_pins.gpio8.into_function(),
        bank0_pins.gpio9.into_function(),
    );
    let mut uart1_peripheral =
        UartPeripheral::new(ctx.device.UART1, uart1_pins, &mut ctx.device.RESETS)
            .enable(
                UartConfig::new(9600_u32.Hz(), DataBits::Eight, None, StopBits::One),
                clocks.peripheral_clock.freq(),
            )
            .unwrap();
    uart1_peripheral.enable_rx_interrupt(); // Make sure we can drive our interrupts
                                            // GPIO10 is the programming pin for HC-12
    let programming = bank0_pins.gpio12.into_push_pull_output();
    // Copy the timer
    let timer = hal::Timer::new_timer1(ctx.device.TIMER1, &mut ctx.device.RESETS, &clocks);
    let mut timer_two = timer.clone();

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
    let radio = radio.into_at_mode().unwrap();
    info!("HC12 in AT Mode");
    timer_two.delay_ms(100);
    let radio = radio.set_baudrate(B9600).unwrap();
    info!("HC12 baudrate set to 9600");
    let radio = radio.set_channel(Channel::Channel1).unwrap();
    info!("HC12 channel set to 1");
    let hc = radio.set_power(Power::P8).unwrap();
    info!("HC12 power set to P8");
    let hc = hc.into_fu3_mode().unwrap();
    info!("HC12 in FU3 Mode");

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

    // Serial Writer Structure
    let serial_console_writer = serial_handler::SerialWriter::new(usb_console_line_sender);
    usb_serial_console_printer::spawn(usb_console_line_receiver).ok();
    usb_console_reader::spawn(usb_console_command_sender).ok();
    #[cfg(debug_assertions)]
    command_handler::spawn(usb_console_command_receiver).ok();
    //radio_flush::spawn().ok();
    state_machine_update::spawn().ok();
    hc12_programmer::spawn().ok();
    incoming_packet_handler::spawn().ok();
    radio_heartbeat::spawn().ok();

    (
        Shared {
            //uart0: uart0_peripheral,
            //uart0_buffer,
            //radio_link,
            ejector_servo,
            usb_device: usb_dev,
            usb_serial: serial,
            serial_console_writer,
            clock_freq_hz: clock_freq.to_Hz(),
            state_machine: EjectorStateMachine::new(),
            blink_status_delay_millis: 1000,
            suspend_packet_handler: false,
            radio: hc,
        },
        Local { led: led_pin },
    )
}
