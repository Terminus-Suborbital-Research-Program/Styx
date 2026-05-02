#![no_std]
#![no_main]
#![warn(missing_docs, clippy::unwrap_used)]

//! TERMINUS RS-X 2026 Elara Ejector Code

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


mod rtic_device {
    pub use rp235x_pac::*;

    pub mod interrupt {
        pub use rp235x_pac::Interrupt::*;
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("Panic: {}", info);
    // Halt the CPU
    hal::halt()
}

/// Tell the Boot ROM about our application
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: rp235x_hal::block::ImageDef = rp235x_hal::block::ImageDef::secure_exe();

#[rtic::app(
    device = crate::rtic_device,
    dispatchers = [PIO2_IRQ_0, PIO2_IRQ_1, DMA_IRQ_0],
)]
mod app {

    use crate::actuators::electromag::ElectroMagnet;
    use crate::actuators::servo::EjectorServo;
    use crate::device_constants::pins::{CamMosfetPin, RBFPin};
    use crate::device_constants::{
        EjectionDetectionPin, JupiterRX, JupiterTX, JupiterUart, OnboardLED, RGBLed, RGBStatus,
        ThermoI2cBus, SAMPLE_COUNT,
    };
    use crate::sd_card::EjectorSdCard;

    use super::*;
    use bin_packets::packets::ApplicationPacket;
    use bin_packets::time::Timestamp;
    use rp235x_hal::gpio::FunctionSio;
    use rp235x_hal::gpio::FunctionSpi;
    use rp235x_hal::Timer;

    use hal::gpio::{self};
    use rtic_sync::portable_atomic::{AtomicBool, Ordering};

    use embedded_hal_bus::spi::ExclusiveDevice;
    use heapless::Deque;
    use rp235x_hal::adc::AdcFifo;
    use rp235x_hal::gpio::{
        bank0::{
            Gpio0, Gpio1, Gpio11, Gpio12, Gpio16, Gpio17, Gpio18, Gpio19, Gpio20, Gpio21, Gpio22,
        },
        Pin,
    };
    use rp235x_hal::pio::SM0;
    use rp235x_hal::gpio::{PullDown, SioOutput, FunctionPio0, bank0::Gpio24};
    use rp235x_hal::pac::{SPI0, PIO0};
    use rp235x_hal::pwm::{Channel, FreeRunning, Slice, A, B};
    use rp235x_hal::spi::{Enabled, Spi, ValidSpiPinout};
    use rp235x_hal::timer::CopyableTimer1;
    use rp235x_hal::uart::UartPeripheral;
    pub const XTAL_FREQ_HZ: u32 = 12_000_000u32;
    use mcp9600::MCP9600;
    use rtic_sync::signal::{SignalReader, SignalWriter};
    use ws2812_pio::Ws2812Direct;


    pub type UART0Bus = UartPeripheral<
        rp235x_hal::uart::Enabled,
        rp235x_hal::pac::UART0,
        (
            Pin<Gpio0, gpio::FunctionUart, gpio::PullDown>,
            Pin<Gpio1, gpio::FunctionUart, gpio::PullDown>,
        ),
    >;

    // TODO: Set proper pins
    pub type EjectorMagnet = ElectroMagnet<
        Pin<Gpio21, gpio::FunctionSioOutput, gpio::PullDown>,
        Pin<Gpio20, gpio::FunctionSioOutput, gpio::PullDown>,
        Pin<Gpio22, gpio::FunctionSioOutput, gpio::PullDown>,
    >;

    pub type EjectorSdSpiPins = (
        Pin<Gpio19, gpio::FunctionSpi, gpio::PullDown>,
        Pin<Gpio16, gpio::FunctionSpi, gpio::PullDown>,
        Pin<Gpio18, gpio::FunctionSpi, gpio::PullDown>,
        //gpio::Pin<gpio::bank0::Gpio17, gpio::FunctionSpi, gpio::PullDown>,
    );
    pub type EjectorSD = EjectorSdCard<
        ExclusiveDevice<
            Spi<
                Enabled,
                SPI0,
                (
                    Pin<Gpio19, FunctionSpi, PullDown>,
                    Pin<Gpio16, FunctionSpi, PullDown>,
                    Pin<Gpio18, FunctionSpi, PullDown>,
                ),
                8,
            >,
            Pin<Gpio17, FunctionSio<SioOutput>, PullDown>,
            Timer<CopyableTimer1>,
        >,
        Timer<CopyableTimer1>,
    >;
    #[shared]
    pub struct Shared {
        pub downlink_packets: Deque<ApplicationPacket, 128>,
        pub samples_buffer: [u16; SAMPLE_COUNT],
        pub sd_card: EjectorSD,
        pub ejection_enabled: bool,
        pub status_config: RGBStatus,
        pub temp_store: Deque<ApplicationPacket, 128>
    }

    #[local]
    pub struct Local {
        // TODO: Add
        // pub onboard_led: OnboardLED,
        pub ejector_servo: EjectorServo,
        pub ejecctor_magnet: EjectorMagnet,
        //pub ejection_pin: EjectionDetectionPin,
        pub rbf_pin: RBFPin,
        pub downlink: JupiterTX,
        // pub arming_led: RedLed,
        // pub packet_led: GreenLed,
        // pub ejection_pin: EjectionDetectionPin,
        pub status_link: JupiterRX,
        pub camera_mosfet: CamMosfetPin,
        pub thermocouple: MCP9600<ThermoI2cBus>,
        // pub rgb_driver: WS2812<RGBLed>,
        pub rgb_driver: Ws2812Direct<
            PIO0,
            SM0,
            Pin<Gpio24, FunctionPio0, PullDown>, 
        >,
        pub ejection_trigger_tx: SignalWriter<'static, ()>,
        pub ejection_trigger_rx: SignalReader<'static, ()>,
    }

    #[init(local = [adc: Option<hal::Adc> = None])]
    fn init(ctx: init::Context) -> (Shared, Local) {
        startup::startup(ctx)
    }

    extern "Rust" {
        // Sequences the ejection
        // ejection pin
        #[task(shared = [ejection_enabled], local = [ ejector_servo, ejecctor_magnet, ejection_trigger_rx],  priority = 1)]
        async fn ejector_sequencer(mut ctx: ejector_sequencer::Context);

        // Sequences cameras activation
        #[task(local = [camera_mosfet], priority = 1)]
        async fn camera_sequencer(mut ctx: camera_sequencer::Context);

        // Heartbeats the main led (and sends packets after arming)
        //  local = [onboard_led],
        #[task(shared = [downlink_packets],  priority = 2)]
        async fn heartbeat(mut ctx: heartbeat::Context);

        #[task( shared = [temp_store], local = [thermocouple], priority = 1)]
        async fn poll_temperature(mut ctx: poll_temperature::Context);

        #[task(shared = [downlink_packets], local = [downlink], priority = 2)]
        async fn downlink_jupiter(mut ctx: downlink_jupiter::Context);

        #[task(shared = [ejection_enabled], local = [rbf_pin], priority = 2)]
        async fn poll_rbf(mut ctx: poll_rbf::Context);

        #[task(shared = [sd_card, temp_store], priority = 2)]
        async fn write_sd_card(mut ctx: write_sd_card::Context);
        // Commands
        // Status for status LED
        #[task(shared = [status_config], local = [status_link, ejection_trigger_tx], priority = 2)]
        async fn rx_from_jupiter(mut ctx: rx_from_jupiter::Context);

        #[task(shared = [status_config], local = [rgb_driver], priority = 1)]
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
