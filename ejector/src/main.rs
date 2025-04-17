#![no_std]
#![no_main]

// Our Modules
pub mod actuators;
pub mod communications;
mod device_constants;
pub mod phases;
pub mod utilities;

// RTIC Tasks
pub mod startup;
pub mod tasks;
pub mod usb_tasks;

use tasks::*;
use usb_tasks::*;

// HAL Access
use rp235x_hal as hal;

use defmt_rtt as _; // global logger

// Monotonics
use rtic_monotonics::rp235x::prelude::*;
rp235x_timer_monotonic!(Mono);

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("Panic: {}", info);
    loop {
        // Halt the CPU
        unsafe {
            hal::sio::spinlock_reset();
        }
    }
}

/// Tell the Boot ROM about our application
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: rp235x_hal::block::ImageDef = rp235x_hal::block::ImageDef::secure_exe();

#[rtic::app(
    device = hal::pac,
    dispatchers = [PIO2_IRQ_0, PIO2_IRQ_1, DMA_IRQ_0],
)]
mod app {
    use crate::device_constants::ListenPin;
    use crate::{actuators::servo::EjectorServo, phases::EjectorStateMachine};

    use super::*;

    use communications::*;

    use hal::gpio::{self, FunctionSio, PullNone, SioOutput};
    use rp235x_hal::uart::UartPeripheral;
    pub const XTAL_FREQ_HZ: u32 = 12_000_000u32;

    use usb_device::{class_prelude::*, prelude::*};
    use usbd_serial::SerialPort;

    use rtic_sync::channel::{Receiver, Sender};
    use serial_handler::{HEAPLESS_STRING_ALLOC_LENGTH, MAX_USB_LINES};

    pub type UART0Bus = UartPeripheral<
        rp235x_hal::uart::Enabled,
        rp235x_hal::pac::UART0,
        (
            gpio::Pin<gpio::bank0::Gpio0, gpio::FunctionUart, gpio::PullDown>,
            gpio::Pin<gpio::bank0::Gpio1, gpio::FunctionUart, gpio::PullDown>,
        ),
    >;

    use crate::hal::timer::CopyableTimer1;
    use hal::gpio::Pin;
    use hal::pac::UART1;
    use hal::uart::Enabled;
    use hal::Timer;
    use hc12_rs::configuration::baudrates::B9600;
    use hc12_rs::ProgrammingPair;
    use hc12_rs::FU3;
    use hc12_rs::HC12;
    use rp235x_hal::gpio::bank0::{Gpio12, Gpio8, Gpio9};
    use rp235x_hal::gpio::FunctionUart;
    use rp235x_hal::gpio::PullDown;
    use rp235x_hal::uart::UartDevice;

    pub type EjectorHC12 = HC12<
        UartPeripheral<
            Enabled,
            UART1,
            (
                Pin<Gpio8, FunctionUart, PullDown>,
                Pin<Gpio9, FunctionUart, PullDown>,
            ),
        >,
        ProgrammingPair<Pin<Gpio12, FunctionSio<SioOutput>, PullDown>, Timer<CopyableTimer1>>,
        FU3<B9600>,
        B9600,
    >;

    pub static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

    #[shared]
    pub struct Shared {
        pub ejector_servo: EjectorServo,
        pub usb_serial: SerialPort<'static, hal::usb::UsbBus>,
        pub usb_device: UsbDevice<'static, hal::usb::UsbBus>,
        pub serial_console_writer: serial_handler::SerialWriter,
        pub clock_freq_hz: u32,
        pub state_machine: EjectorStateMachine,
        pub blink_status_delay_millis: u64,
        pub suspend_packet_handler: bool,
        pub radio: EjectorHC12,
        pub ejection_pin: ListenPin,
    }

    #[local]
    pub struct Local {
        pub led: gpio::Pin<gpio::bank0::Gpio25, FunctionSio<SioOutput>, PullNone>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        startup::startup(ctx)
    }

    extern "Rust" {
        // Takes care of receiving incoming packets
        #[task(shared = [state_machine, suspend_packet_handler], priority = 1)]
        async fn incoming_packet_handler(mut ctx: incoming_packet_handler::Context);

        // State machine update
        #[task(shared = [state_machine, serial_console_writer, ejector_servo, blink_status_delay_millis, ejection_pin], priority = 1)]
        async fn state_machine_update(mut ctx: state_machine_update::Context);

        // Heartbeats the main led
        #[task(local = [led], shared = [blink_status_delay_millis, radio], priority = 2)]
        async fn heartbeat(mut ctx: heartbeat::Context);

        // Heartbeats the radio
        #[task(shared = [radio], priority = 2)]
        async fn radio_heartbeat(mut ctx: radio_heartbeat::Context);

        // Reads from the USB console
        #[task(priority = 3, shared = [usb_device, usb_serial, serial_console_writer])]
        async fn usb_console_reader(
            mut ctx: usb_console_reader::Context,
            mut command_sender: Sender<
                'static,
                heapless::String<HEAPLESS_STRING_ALLOC_LENGTH>,
                MAX_USB_LINES,
            >,
        );

        // Writes to the USB console
        #[task(priority = 3, shared = [usb_device, usb_serial])]
        async fn usb_serial_console_printer(
            mut ctx: usb_serial_console_printer::Context,
            mut reciever: Receiver<
                'static,
                heapless::String<HEAPLESS_STRING_ALLOC_LENGTH>,
                MAX_USB_LINES,
            >,
        );

        // Command Handler for USB Console
        #[task(shared=[serial_console_writer, clock_freq_hz, ejector_servo, state_machine, suspend_packet_handler], priority = 2)]
        async fn command_handler(
            mut ctx: command_handler::Context,
            mut reciever: Receiver<
                'static,
                heapless::String<HEAPLESS_STRING_ALLOC_LENGTH>,
                MAX_USB_LINES,
            >,
        );

        // Updates the radio module on the serial interrupt
        #[task(binds = UART1_IRQ, shared = [serial_console_writer])]
        fn uart_interrupt(mut ctx: uart_interrupt::Context);

        // // Radio Flush Task
        // #[task(priority = 1)]
        // async fn radio_flush(mut ctx: radio_flush::Context);

        // An async task to program the HC12 module
        #[task(shared = [serial_console_writer, suspend_packet_handler], priority = 3)]
        async fn hc12_programmer(mut ctx: hc12_programmer::Context);
    }
}
