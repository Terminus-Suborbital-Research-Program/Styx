#![no_std]
#![no_main]
#![warn(missing_docs)]

//! TERMINUS RS-X 2026 Elara Ejector Code

// Our Modules
pub mod actuators;

mod device_constants;
pub mod sd_card;

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

    use crate::actuators::electromag::ElectroMagnet;
    use crate::actuators::servo::EjectorServo;
    use crate::device_constants::pins::{CamMosfetPin, RBFPin};
    use crate::device_constants::{
        EjectionDetectionPin, GreenLed, JupiterUart, OnboardLED, RedLed, ThermoI2cBus, SAMPLE_COUNT,
        RGBStatus, RGBLed, JupiterRX, JupiterTX, 
    };

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
    use ws2812_rs::WS2812;

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
        gpio::Pin<gpio::bank0::Gpio21, gpio::FunctionSioOutput, gpio::PullDown>,
        gpio::Pin<gpio::bank0::Gpio20, gpio::FunctionSioOutput, gpio::PullDown>,
        gpio::Pin<gpio::bank0::Gpio22, gpio::FunctionSioOutput, gpio::PullDown>,
    >;

    #[shared]
    pub struct Shared {
        pub downlink_packets: Deque<ApplicationPacket, 128>,
        pub samples_buffer: [u16; SAMPLE_COUNT],
        pub ejection_enabled: bool,
        pub status_config: RGBStatus,
    }

    #[local]
    pub struct Local {
        // TODO: Add
        pub onboard_led: OnboardLED,
        pub ejector_servo: EjectorServo,
        pub ejecctor_magnet: EjectorMagnet,
        pub arming_led: RedLed,
        pub packet_led: GreenLed,
        pub ejection_pin: EjectionDetectionPin,
        pub rbf_pin: RBFPin,
        pub downlink: JupiterTX,
        pub status_link: JupiterRX,
        pub camera_mosfet: CamMosfetPin,
        pub thermocouple: MCP9600<ThermoI2cBus>,
        pub rgb_driver: WS2812<RGBLed>,
    }

    #[init(local = [adc: Option<hal::Adc> = None])]
    fn init(ctx: init::Context) -> (Shared, Local) {
        startup::startup(ctx)
    }

    extern "Rust" {
        // Sequences the ejection
        #[task(shared = [ejection_enabled], local = [ejection_pin, arming_led, ejector_servo, ejecctor_magnet],  priority = 1)]
        async fn ejector_sequencer(mut ctx: ejector_sequencer::Context);

        // Sequences cameras activation
        #[task(local = [camera_mosfet], priority = 1)]
        async fn camera_sequencer(mut ctx: camera_sequencer::Context);

        // Heartbeats the main led (and sends packets after arming)
        #[task(shared = [downlink_packets], local = [onboard_led], priority = 1)]
        async fn heartbeat(mut ctx: heartbeat::Context);

        #[task( local = [thermocouple], priority = 1)]
        async fn poll_temperature(mut ctx: poll_temperature::Context);

        #[task(shared = [downlink_packets], local = [downlink], priority = 1)]
        async fn downlink_jupiter(mut ctx: downlink_jupiter::Context);

        #[task(shared = [ejection_enabled], local = [rbf_pin], priority = 2)]
        async fn poll_rbf(mut ctx: poll_rbf::Context);

        // Commands
        // Status for status LED
        #[task(shared = [status_config], local = [status_link], priority = 2)]
        async fn rx_from_jupiter(mut ctx: rx_from_jupiter::Context);

        #[task(shared = [status_config], local = [rgb_driver], priority = 2)]
        async fn set_rgb_status(mut ctx: set_rgb_status::Context);



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
