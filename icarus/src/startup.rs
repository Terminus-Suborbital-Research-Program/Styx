//use bme280::i2c::BME280;
use embedded_hal::digital::OutputPin;
use fugit::RateExtU32;
use fugit::RateExtU64;
use rp235x_hal::clocks::init_clocks_and_plls;
use rp235x_hal::gpio::PullNone;
use rp235x_hal::pwm::Slices;
use rp235x_hal::uart::DataBits;
use rp235x_hal::uart::StopBits;
use rp235x_hal::uart::UartConfig;
use rp235x_hal::uart::UartPeripheral;
use rp235x_hal::Clock;
use rp235x_hal::Sio;
use rp235x_hal::Watchdog;
use rp235x_hal::I2C;
use rtic_sync::make_channel;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::StringDescriptors;
use usb_device::device::UsbDeviceBuilder;
use usb_device::device::UsbVidPid;
use usbd_serial::SerialPort;

use crate::actuators::motor::Motor;
use crate::actuators::servo::HOLDING_ANGLE;
use crate::actuators::servo::{EjectionServoMosfet, LockingServoMosfet, Servo};
use crate::actuators::PWM2a;

use crate::app::*;
use crate::communications::hc12::HC12;
use crate::communications::link_layer::Device;
use crate::communications::link_layer::LinkLayerDevice;
use crate::communications::serial_handler;
use crate::communications::serial_handler::HeaplessString;
use crate::communications::serial_handler::MAX_USB_LINES;
use crate::hal;
use crate::Mono;
use crate::ALLOCATOR;
use crate::HEAP_MEMORY;
use crate::{DelayTimer, I2CMainBus};

use embedded_hal_bus::util::AtomicCell;

pub fn startup(mut ctx: init::Context) -> (Shared, Local) {
    // Reset the spinlocks - this is skipped by soft-reset
    unsafe {
        hal::sio::spinlock_reset();
    }

    // Set up the global allocator
    unsafe {
        ALLOCATOR
            .lock()
            .init(HEAP_MEMORY.as_ptr() as *mut u8, HEAP_MEMORY.len());
    }

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

    // Use pin 14 (GPIO10) as the HC12 configuration pin
    let hc12_configure_pin = bank0_pins.gpio10.into_push_pull_output();
    let hc12 = HC12::new(uart1_peripheral, hc12_configure_pin).unwrap();
    let radio_link = LinkLayerDevice {
        device: hc12,
        me: Device::Ejector,
    };

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
    let mut ejection_servo = Servo::new(channel_a, channel_pin, mosfet_pin);
    ejection_servo.set_angle(90);

    // Locking servo
    let mut locking_pwm = pwm_slices.pwm1;
    locking_pwm.enable();
    locking_pwm.set_div_int(48);
    let mut locking_mosfet_pin: LockingServoMosfet = bank0_pins.gpio3.into_push_pull_output();
    locking_mosfet_pin.set_low().unwrap();
    let mut locking_channel_a = locking_pwm.channel_a;
    let locking_channel_pin = locking_channel_a.output_to(bank0_pins.gpio2);
    locking_channel_a.set_enabled(true);
    let mut locking_servo = Servo::new(locking_channel_a, locking_channel_pin, locking_mosfet_pin);
    locking_servo.set_angle(HOLDING_ANGLE);

    // Motor Initialization
    let mut motor_xy_pwm = pwm_slices.pwm2;
    motor_xy_pwm.enable();
    motor_xy_pwm.set_top(65534 / 2);
    motor_xy_pwm.set_div_int(1);
    let mut motor_x_channel: PWM2a = motor_xy_pwm.channel_a;
    let motor_x_channel_pin = motor_x_channel.output_to(bank0_pins.gpio4);
    let mut motor_x = Motor::new(motor_x_channel, motor_x_channel_pin);
    motor_x.set_speed(0);

    // Sensors
    // Init I2C pins
    let sda_pin = bank0_pins.gpio14.reconfigure();
    let scl_pin = bank0_pins.gpio15.reconfigure();

    let i2c1: I2CMainBus = I2C::new_controller(
        ctx.device.I2C1,
        sda_pin,
        scl_pin,
        RateExtU32::kHz(400),
        &mut ctx.device.RESETS,
        clocks.system_clock.freq(),
    );

    let i2c_bus = ctx.local.i2c_main_bus.write(AtomicCell::new(i2c1));

    let delay: DelayTimer =
        rp235x_hal::Timer::new_timer1(ctx.device.TIMER1, &mut ctx.device.RESETS, &clocks);
    //let mut bme280 = BME280::new_primary(AtomicDevice::new(i2c_bus));
    //bme280.init(&mut delay);

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

    usb_serial_console_printer::spawn(usb_console_line_receiver).ok();
    usb_console_reader::spawn(usb_console_command_sender).ok();
    command_handler::spawn(usb_console_command_receiver).ok();
    radio_flush::spawn().ok();
    incoming_packet_handler::spawn().ok();
    sample_sensors::spawn().ok();

    // Serial Writer Structure
    let serial_console_writer = serial_handler::SerialWriter::new(usb_console_line_sender);

    (
        Shared {
            //uart0: uart0_peripheral,
            //uart0_buffer,
            radio_link,
            ejector_driver: ejection_servo,
            locking_driver: locking_servo,
            usb_device: usb_dev,
            usb_serial: serial,
            serial_console_writer,
            clock_freq_hz: clock_freq.to_Hz(),
            software_delay: delay,
            //env_sensor: bme280
        },
        Local { led: led_pin },
    )
}
