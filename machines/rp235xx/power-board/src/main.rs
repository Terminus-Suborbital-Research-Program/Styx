#![warn(missing_docs)]
#![no_std]
#![no_main]


pub mod device_constants;
pub mod startup;
pub mod task;
// HAL Access
use rp235x_hal as hal;

use defmt_rtt as _; // global logger

use crate::startup::SAMPLE_COUNT;

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


    use crate::device_constants::pins::JupiterI2c;

    use super::*;

    use bin_packets::packets::ApplicationPacket;
    use bin_packets::time::Timestamp;

    use hal::gpio::{self};

    use heapless::Deque;
    use rp235x_hal::adc::AdcFifo;
    use rp235x_hal::pwm::{Channel, FreeRunning, Slice, A, B};
    use rp235x_hal::uart::UartPeripheral;
    pub const XTAL_FREQ_HZ: u32 = 12_000_000u32;
    use mcp9600::MCP9600;

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
        pub downlink_packets: Deque<ApplicationPacket, 128>,
        pub samples_buffer: [u16; SAMPLE_COUNT],
    }

    #[local]
    pub struct Local {
        pub jupiter_i2c: JupiterI2c,
    }

    #[init(local = [adc: Option<hal::Adc> = None])]
    fn init(ctx: init::Context) -> (Shared, Local) {
        startup::startup(ctx)
    }

    extern "Rust" {
        
        
        // #[task(binds = ADC_IRQ_FIFO, priority = 3, shared = [samples_buffer], local = [ counter: usize = 1])]
        // fn adc_irq(mut ctx: adc_irq::Context);

        #[task(priority = 1, local = [jupiter_i2c])]
        async fn power_switch(mut ctx: power_switch::Context);
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

