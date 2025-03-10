// Specifies that the standard library is not used
#![no_std]
#![no_main]

// Our Modules
pub mod actuators;
pub mod communications;
pub mod sensors;
pub mod startup;
pub mod tasks;
pub mod usb_commands;
pub mod usb_io;
pub mod utilities;

#[allow(dead_code)]
use panic_halt as _;

// We require an allocator for some heap stuff - unfortunatly bincode serde
// doesn't have support for heapless vectors yet
extern crate alloc;
use linked_list_allocator::LockedHeap;

use crate::tasks::*;
use crate::usb_commands::*;
use crate::usb_io::*;
use core::mem::MaybeUninit;
//use bme280::i2c::BME280;
use embedded_hal_bus::util::AtomicCell;
use icarus::{DelayTimer, I2CMainBus};

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();
static mut HEAP_MEMORY: [u8; 1024 * 64] = [0; 1024 * 64];

use panic_halt as _;

// HAL Access
#[cfg(feature = "rp2350")]
use rp235x_hal as hal;

// Monotonics
#[cfg(feature = "rp2350")]
use rtic_monotonics::rp235x::prelude::*;
#[cfg(feature = "rp2350")]
rp235x_timer_monotonic!(Mono);

/// Tell the Boot ROM about our application
#[link_section = ".start_block"]
#[used]
#[cfg(feature = "rp2350")]
pub static IMAGE_DEF: rp235x_hal::block::ImageDef = rp235x_hal::block::ImageDef::secure_exe();

#[rtic::app(
    device = hal::pac,
    dispatchers = [PIO2_IRQ_0, PIO2_IRQ_1, DMA_IRQ_0],
    peripherals = true,
)]
mod app {
    use crate::{
        actuators::servo::{EjectionServo, LockingServo},
        communications::hc12::{UART1Bus, GPIO10},
    };

    use super::*;

    use communications::{link_layer::LinkLayerDevice, *};

    use hal::gpio::{self, FunctionSio, PullNone, SioOutput};
    use rp235x_hal::uart::UartPeripheral;
    pub const XTAL_FREQ_HZ: u32 = 12_000_000u32;

    use usb_device::{class_prelude::*, prelude::*};

    use hc12::HC12;

    use rtic_sync::channel::{Receiver, Sender};
    use serial_handler::{HEAPLESS_STRING_ALLOC_LENGTH, MAX_USB_LINES};
    use usbd_serial::SerialPort;

    pub type UART0Bus = UartPeripheral<
        rp235x_hal::uart::Enabled,
        rp235x_hal::pac::UART0,
        (
            gpio::Pin<gpio::bank0::Gpio0, gpio::FunctionUart, gpio::PullDown>,
            gpio::Pin<gpio::bank0::Gpio1, gpio::FunctionUart, gpio::PullDown>,
        ),
    >;

    pub static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;
    #[shared]
    pub struct Shared {
        //uart0: UART0Bus,
        //uart0_buffer: heapless::String<HEAPLESS_STRING_ALLOC_LENGTH>,
        pub ejector_driver: EjectionServo,
        pub locking_driver: LockingServo,
        pub radio_link: LinkLayerDevice<HC12<UART1Bus, GPIO10>>,
        pub usb_serial: SerialPort<'static, hal::usb::UsbBus>,
        pub usb_device: UsbDevice<'static, hal::usb::UsbBus>,
        pub serial_console_writer: serial_handler::SerialWriter,
        pub clock_freq_hz: u32,
        pub software_delay: DelayTimer,
        //pub env_sensor: BME280<AtomicDevice<'static,I2CMainBus>>
    }

    #[local]
    pub struct Local {
        pub led: gpio::Pin<gpio::bank0::Gpio25, FunctionSio<SioOutput>, PullNone>,
    }

    #[init(
        local=[
            // Task local initialized resources are static
            // Here we use MaybeUninit to allow for initialization in init()
            // This enables its usage in driver initialization
            i2c_main_bus: MaybeUninit<AtomicCell<I2CMainBus>> = MaybeUninit::uninit(),
        ]
    )]
    fn init(ctx: init::Context) -> (Shared, Local) {
        startup::startup(ctx)
    }

    extern "Rust" {
        // Heartbeats the main led
        #[task(local = [led], priority = 2)]
        async fn heartbeat(ctx: heartbeat::Context);

        // Takes care of incoming packets
        #[task(shared = [radio_link, serial_console_writer], priority = 1)]
        async fn incoming_packet_handler(mut ctx: incoming_packet_handler::Context);
    }

    extern "Rust" {
        // USB Console Reader
        #[task(priority = 2, shared = [usb_device, usb_serial, serial_console_writer])]
        async fn usb_console_reader(
            mut ctx: usb_console_reader::Context,
            mut command_sender: Sender<
                'static,
                heapless::String<HEAPLESS_STRING_ALLOC_LENGTH>,
                MAX_USB_LINES,
            >,
        );

        // USB Console Printer
        #[task(priority = 2, shared = [usb_device, usb_serial])]
        async fn usb_serial_console_printer(
            mut ctx: usb_serial_console_printer::Context,
            mut reciever: Receiver<
                'static,
                heapless::String<HEAPLESS_STRING_ALLOC_LENGTH>,
                MAX_USB_LINES,
            >,
        );

        // Command Handler
        #[task(shared=[serial_console_writer, radio_link, clock_freq_hz, ejector_driver, locking_driver], priority = 2)]
        async fn command_handler(
            mut ctx: command_handler::Context,
            mut reciever: Receiver<
                'static,
                heapless::String<HEAPLESS_STRING_ALLOC_LENGTH>,
                MAX_USB_LINES,
            >,
        );

        // Updates the radio module on the serial interrupt
        #[task(binds = UART1_IRQ, shared = [radio_link, serial_console_writer])]
        fn uart_interrupt(mut ctx: uart_interrupt::Context);

        // Radio Flush Task
        #[task(shared = [radio_link], priority = 1)]
        async fn radio_flush(mut ctx: radio_flush::Context);

        #[task(shared = [serial_console_writer, software_delay], priority = 3)]
        async fn sample_sensors(mut ctx: sample_sensors::Context);

    }
}
