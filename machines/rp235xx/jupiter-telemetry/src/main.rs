// Specifies that the standard library is not used
#![no_std]
#![no_main]

// Our Modules
mod device_constants;
mod peripherals;
mod sensors;
mod startup;
mod tasks;

use defmt_rtt as _; // global logger

use crate::tasks::*;
use core::mem::MaybeUninit;

// Sensors
use bme280::AsyncBME280;
use bmi323::AsyncBmi323;
use bmm350::AsyncBmm350;
use bmp5::i2c::Bmp5;
use ina260_terminus::AsyncINA260;

// Busses
use rtic_sync::arbiter::i2c::ArbiterDevice;

/// Lets us know when we panic
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("Panic: {}", info);
    // Better panic
    hal::halt();
}

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
    use crate::device_constants::{
        AvionicsI2cBus, DownlinkBuffer, ComputeI2cBus,
    };

    use super::*;

    use bin_packets::{phases::IcarusPhase, time::Timestamp};

    use hal::gpio::{self, FunctionSio, PullNone, SioOutput};
    use rp235x_hal::{
        gpio::{bank0::Gpio4, Pin, PullDown, SioInput},
        uart::UartPeripheral,
    };
    pub const XTAL_FREQ_HZ: u32 = 12_000_000u32;

    use rtic_sync::{arbiter::Arbiter, signal::Signal};

    pub type UART0Bus = UartPeripheral<
        rp235x_hal::uart::Enabled,
        rp235x_hal::pac::UART0,
        (
            gpio::Pin<gpio::bank0::Gpio0, gpio::FunctionUart, gpio::PullDown>,
            gpio::Pin<gpio::bank0::Gpio1, gpio::FunctionUart, gpio::PullDown>,
        ),
    >;

    // pub static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;
    #[shared]
    pub struct Shared {
        pub data: DownlinkBuffer,
    }

    #[local]
    pub struct Local {
        pub led: gpio::Pin<gpio::bank0::Gpio25, FunctionSio<SioOutput>, PullNone>,
        pub bmm350: AsyncBmm350<ArbiterDevice<'static, AvionicsI2cBus>, Mono>,
        pub bmi323: AsyncBmi323<ArbiterDevice<'static, AvionicsI2cBus>, Mono>,
        pub bme280: AsyncBME280<ArbiterDevice<'static, AvionicsI2cBus>, Mono>,
        pub bmp5: Bmp5<ArbiterDevice<'static, AvionicsI2cBus>, Mono>,
    }

    #[init(
        local=[
            // Task local initialized resources are static Here we use MaybeUninit to allow for initialization in init()
            // This enables its usage in driver initialization
            i2c_avionics_bus: MaybeUninit<Arbiter<AvionicsI2cBus>> = MaybeUninit::uninit(),
            i2c_compute_bus: MaybeUninit<Arbiter<ComputeI2cBus>> = MaybeUninit::uninit(),
            esc_state_signal: MaybeUninit<Signal<IcarusPhase>> = MaybeUninit::uninit(),
        ]
    )]
    fn init(ctx: init::Context) -> (Shared, Local) {
        startup::startup(ctx)
    }

    extern "Rust" {
        // Heartbeats the main led
        #[task(local = [led], shared = [data], priority = 1)]
        async fn heartbeat(ctx: heartbeat::Context);


        #[task(local = [bme280, bmi323, bmm350, bmp5], shared = [data], priority = 2)]
        async fn sample_sensors(
            mut ctx: sample_sensors::Context,
            avionics_i2c: &'static Arbiter<AvionicsI2cBus>,
        );

  
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
