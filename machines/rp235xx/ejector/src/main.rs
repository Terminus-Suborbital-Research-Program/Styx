#![no_std]
#![no_main]
#![warn(missing_docs)]

//! Entry point for the ejector flight code for ELARA

// Our Modules
pub mod actuators;

mod device_constants;

// Guard module
pub mod guard;

// RTIC Tasks
pub mod startup;
pub mod tasks;

use tasks::*;

use bin_packets::packets::testing;

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
//#[cfg(feature = "testing")]
pub struct EjectorTests {
    sanity_check: testing::TestStatus,
    rbf_test: testing::TestStatus,
    ejection_test: testing::TestStatus,
    uart0_test: testing::TestStatus,
}

//#[cfg(feature = "testing")]
impl EjectorTests {
    pub fn new() -> Self {
        Self {
            sanity_check: testing::TestStatus::NotTested,
            rbf_test: testing::TestStatus::NotTested,
            ejection_test: testing::TestStatus::NotTested,
            uart0_test: testing::TestStatus::NotTested,
        }
    }
}

pub const TESTING_CHANNEL_CAPACITY: usize = 10;

/// Tell the Boot ROM about our application
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: rp235x_hal::block::ImageDef = rp235x_hal::block::ImageDef::secure_exe();

#[rtic::app(
    device = hal::pac,
    dispatchers = [PIO2_IRQ_0, PIO2_IRQ_1, DMA_IRQ_0],
)]
mod app {

    use crate::actuators::electromag::ElectroMagnet;
    use crate::actuators::servo::EjectorServo;
    use crate::device_constants::pins::CamMosfetPin;
    use crate::device_constants::{
        EjectionDetectionPin, GreenLed, JupiterUart, OnboardLED, RedLed, SAMPLE_COUNT, ThermoI2cBus
    };

    use super::*;

    use bin_packets::packets::ApplicationPacket;
    use bin_packets::time::Timestamp;

    use hal::gpio::{self};

    use heapless::Deque;
    use rp235x_hal::adc::AdcFifo;
    use rp235x_hal::pwm::{Channel, FreeRunning, Slice, A, B};
    use rp235x_hal::uart::UartPeripheral;
    use rtic_sync::channel::{Receiver, Sender};
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

    // TODO: Set proper pins
    pub type EjectorMagnet = ElectroMagnet<
        Channel<Slice<rp235x_hal::pwm::Pwm2, FreeRunning>, A>,
        Channel<Slice<rp235x_hal::pwm::Pwm2, FreeRunning>, B>,
        gpio::Pin<gpio::bank0::Gpio22, gpio::FunctionSioOutput, gpio::PullDown>,
    >;

    #[shared]
    pub struct Shared {
        pub downlink_packets: Deque<ApplicationPacket, 128>,
        pub samples_buffer: [u16; SAMPLE_COUNT],
        pub tests: EjectorTests,
    }

    #[local]
    pub struct Local {
        // TODO: Add
        pub onboard_led: OnboardLED,
        pub ejector_servo: EjectorServo,
        pub ejecctor_magnet: EjectorMagnet,
        pub arming_led: RedLed,
        pub num_var: u8,
        pub packet_led: GreenLed,
        pub ejection_pin: EjectionDetectionPin,
        pub downlink: JupiterUart,
        pub camera_mosfet: CamMosfetPin,
        pub thermocouple: MCP9600<ThermoI2cBus>,
    }

    #[init(local = [adc: Option<hal::Adc> = None])]
    fn init(ctx: init::Context) -> (Shared, Local) {
        startup::startup(ctx)
    }

    extern "Rust" {
        // Sequences the ejection
        #[task(local = [ejection_pin, arming_led, ejector_servo, ejecctor_magnet], shared = [tests], priority = 1)]
        async fn ejector_sequencer(mut ctx: ejector_sequencer::Context);

        // Sequences cameras activation
        #[task(local = [camera_mosfet], priority = 1)]
        async fn camera_sequencer(mut ctx: camera_sequencer::Context);

        #[task(shared = [tests], priority = 1)]
        async fn testing_handler(
            ctx: testing_handler::Context<'_>,
            mut receiver: Option<Receiver<'static, bool, TESTING_CHANNEL_CAPACITY>>,
        );

        #[task(priority = 1, local = [num_var])]
        async fn jupiter_read(mut ctx: jupiter_read::Context);

        #[task(priority = 1)]
        async fn jupiter_write(mut ctx: jupiter_write::Context);

        // Heartbeats the main led (and sends packets after arming)
        #[task(shared = [downlink_packets], local = [onboard_led], priority = 1)]
        async fn heartbeat(mut ctx: heartbeat::Context);

        #[task( local = [thermocouple], priority = 1)]
        async fn poll_temperature(mut ctx: poll_temperature::Context);

        #[task(shared = [downlink_packets], local = [downlink], priority = 1)]
        async fn downlink_jupiter(mut ctx: downlink_jupiter::Context);

        
        // #[task(binds = ADC_IRQ_FIFO, priority = 3, shared = [samples_buffer], local = [ counter: usize = 1])]
        // fn adc_irq(mut ctx: adc_irq::Context);

        // #[task(priority = 2, shared = [samples_buffer, downlink_packets])]
        // async fn geiger_calculator(mut ctx: geiger_calculator::Context);
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
