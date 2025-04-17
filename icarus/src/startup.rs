use bin_packets::IcarusPhase;
use cortex_m::delay::Delay;
use defmt::error;
use defmt::info;
use embedded_hal::digital::OutputPin;
use fugit::RateExtU32;
use fugit::RateExtU64;
use mcf8316c_rs::controller::MotorController;
use rp235x_hal::clocks::init_clocks_and_plls;
use rp235x_hal::gpio::FunctionI2C;
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
use rtic_monotonics::Monotonic;
use rtic_sync::arbiter::i2c::ArbiterDevice;
use rtic_sync::arbiter::Arbiter;
use rtic_sync::make_signal;
use rtic_sync::signal::Signal;
use usb_device::bus::UsbBusAllocator;
use usbd_serial::SerialPort;
// use usb_device::bus::UsbBusAllocator;
// use usbd_serial::SerialPort;

use crate::actuators::servo::HOLDING_ANGLE;
use crate::actuators::servo::{EjectionServoMosfet, LockingServoMosfet, Servo};

use crate::app::*;
use crate::communications::hc12::HC12;
use crate::communications::link_layer::Device;
use crate::communications::link_layer::LinkLayerDevice;
use crate::device_constants::pins::{AvionicsI2CSclPin, AvionicsI2CSdaPin};
use crate::device_constants::pins::{EscI2CSclPin, EscI2CSdaPin};
use crate::device_constants::IcarusStateMachine;
use crate::device_constants::MotorI2cBus;
use crate::hal;
use crate::peripherals::async_i2c::AsyncI2c;
use crate::phases::StateMachine;
use crate::phases::StateMachineListener;
use crate::Mono;
use crate::ALLOCATOR;
use crate::HEAP_MEMORY;
use crate::{DelayTimer, I2CMainBus};

// Sensors
use bme280_rs::{AsyncBme280, Bme280, Configuration, Oversampling, SensorMode};
use ina260_terminus::{AsyncINA260, Register as INA260Register};

// Logs our time for demft
defmt::timestamp!("{=u64:us}", {
    Mono::now().duration_since_epoch().to_nanos()
});

pub fn startup(mut ctx: init::Context) -> (Shared, Local) {
    // Reset the spinlocks - this is skipped by soft-reset
    unsafe {
        hal::sio::spinlock_reset();
    }

    // Set up the global allocator, have to do unsafe shit
    #[allow(static_mut_refs)]
    unsafe {
        ALLOCATOR
            .lock()
            .init(HEAP_MEMORY.as_ptr() as *mut u8, HEAP_MEMORY.len());
    }

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
    );

    let clocks = match clocks {
        Ok(clocks) => clocks,
        Err(err) => {
            error!("Failed to initialize clocks: {:?}", err);
            panic!();
        }
    };

    info!("Good morning sunshine! Icarus is awake!");

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

    // Sensors
    // Init I2C pins
    let motor_sda_pin: Pin<EscI2CSdaPin, FunctionI2C, PullUp> = bank0_pins.gpio18.reconfigure();
    let motor_scl_pin: Pin<EscI2CSclPin, FunctionI2C, PullUp> = bank0_pins.gpio19.reconfigure();

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
    let motor_controller = MotorController::new(0x01, ArbiterDevice::new(motor_i2c_arbiter));

    let avionics_sda_pin: Pin<AvionicsI2CSdaPin, FunctionI2C, PullUp> =
        bank0_pins.gpio16.reconfigure();
    let avionics_scl_pin: Pin<AvionicsI2CSclPin, FunctionI2C, PullUp> =
        bank0_pins.gpio17.reconfigure();

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
    let mut bme280 =
        AsyncBme280::new_with_address(ArbiterDevice::new(avionics_i2c_arbiter), 0x77, Mono);

    // State machine
    let mut state_machine: IcarusStateMachine = StateMachine::new();
    let esc_state_signal: Signal<IcarusPhase> = Signal::new();
    ctx.local.esc_state_signal.write(esc_state_signal);

    let r = unsafe { ctx.local.esc_state_signal.assume_init_ref() };
    let (writer, reader) = r.split();
    state_machine.add_channel(writer).ok();
    let esc_listener = StateMachineListener::new(reader);

    let serial = SerialPort::new(usb_bus_ref);
    let mut ina260_1 = AsyncINA260::new(ArbiterDevice::new(motor_i2c_arbiter), 32_u8, Mono);
    let mut ina260_2 = AsyncINA260::new(ArbiterDevice::new(motor_i2c_arbiter), 33_u8, Mono);
    let mut ina260_3 = AsyncINA260::new(ArbiterDevice::new(motor_i2c_arbiter), 34_u8, Mono);

    info!("Peripherals initialized, spawning tasks...");
    // heartbeat::spawn().ok();
    radio_flush::spawn().ok();
    incoming_packet_handler::spawn().ok();
    motor_drivers::spawn(motor_i2c_arbiter, esc_listener).ok();
    sample_sensors::spawn(avionics_i2c_arbiter).ok();
    inertial_nav::spawn().ok();
    info!("Tasks spawned!");

    (
        Shared {
            radio_link,
            ejector_driver: ejection_servo,
            locking_driver: locking_servo,
            clock_freq_hz: clock_freq.to_Hz(),
            state_machine,
        },
        Local {
            led: led_pin,
            bme280: bme280,
            ina260_1,
            ina260_2,
            ina260_3,
        },
    )
}
