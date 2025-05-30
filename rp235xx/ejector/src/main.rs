#![no_std]
#![no_main]

// Our Modules
pub mod actuators;

mod device_constants;

// Guard module
pub mod guard;

// RTIC Tasks
pub mod startup;
pub mod tasks;

use tasks::*;

// HAL Access
use rp235x_hal as hal;

use defmt_rtt as _; // global logger

// Monotonics
use rtic_monotonics::rp235x::prelude::*;
rp235x_timer_monotonic!(Mono);

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("Panic: {}", info);
    // Halt the CPU
    hal::halt();
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
    use crate::device_constants::packets::RadioInterface;

    use crate::actuators::servo::EjectorServo;
    use crate::device_constants::pins::CamMosfetPin;
    use crate::device_constants::{
        EjectionDetectionPin, GreenLed, JupiterUart, OnboardLED, RedLed,
    };

    use super::*;

    use bin_packets::packets::ApplicationPacket;
    use bin_packets::time::Timestamp;

    use hal::gpio::{self};

    use heapless::Deque;
    use rp235x_hal::uart::UartPeripheral;
    pub const XTAL_FREQ_HZ: u32 = 12_000_000u32;

    pub type UART0Bus = UartPeripheral<
        rp235x_hal::uart::Enabled,
        rp235x_hal::pac::UART0,
        (
            gpio::Pin<gpio::bank0::Gpio0, gpio::FunctionUart, gpio::PullDown>,
            gpio::Pin<gpio::bank0::Gpio1, gpio::FunctionUart, gpio::PullDown>,
        ),
    >;

    #[shared]
    pub struct Shared {
        pub downlink_packets: Deque<ApplicationPacket, 16>,
        pub radio: RadioInterface,
    }

    #[local]
    pub struct Local {
        pub onboard_led: OnboardLED,
        pub ejector_servo: EjectorServo,
        pub arming_led: RedLed,
        pub packet_led: GreenLed,
        pub ejection_pin: EjectionDetectionPin,
        pub downlink: JupiterUart,
        pub camera_mosfet: CamMosfetPin,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        startup::startup(ctx)
    }

    extern "Rust" {
        // Sequences the ejection
        #[task(local = [ejection_pin, arming_led, ejector_servo],  priority = 2)]
        async fn ejector_sequencer(mut ctx: ejector_sequencer::Context);

        // Sequences cameras activation
        #[task(local = [camera_mosfet], priority = 2)]
        async fn camera_sequencer(mut ctx: camera_sequencer::Context);

        // Heartbeats the main led (and sends packets after arming)
        #[task(shared = [radio, downlink_packets], local = [onboard_led], priority = 2)]
        async fn heartbeat(mut ctx: heartbeat::Context);

        // Reads incoming packets from the radio
        #[task(local = [downlink, packet_led], shared = [radio, downlink_packets], priority = 3)]
        async fn radio_read(mut ctx: radio_read::Context);

        // Updates the radio module on the serial interrupt
        #[task(binds = UART1_IRQ, shared = [radio])]
        fn uart_interrupt(mut ctx: uart_interrupt::Context);
    }

    /// Returns the current time in nanoseconds since power-on
    pub fn epoch_ns() -> u64 {
        Mono::now().duration_since_epoch().to_nanos()
    }

    /// Returns the current time as a timestamp
    pub fn now_timestamp() -> Timestamp {
        Timestamp::new(epoch_ns())
    }
}
