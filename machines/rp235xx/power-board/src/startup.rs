#![warn(missing_docs)]

use defmt::{info, warn};
use embedded_hal::digital::OutputPin;
use fugit::RateExtU32;
use heapless::Deque;
use rp235x_hal::adc::AdcPin;
use rp235x_hal::clocks::init_clocks_and_plls;
use rp235x_hal::gpio::{FunctionI2C, FunctionI2c, FunctionSio, FunctionUart, PinState, PullNone, SioInput};
use rp235x_hal::pwm::Slices;
use rp235x_hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use rp235x_hal::{Clock, Sio, Watchdog};
use rtic_monotonics::Monotonic;
use crate::tasks;

use mcp9600::{
    ADCResolution, BurstModeSamples, ColdJunctionResolution, DeviceAddr, 
    FilterCoefficient, MCP9600, ShutdownMode, ThermocoupleType
};
use rp235x_hal::i2c::I2C;
// use rp235x_hal::timer::monotonic::Monotonic;

pub const SAMPLE_COUNT :usize= 128;


use crate::device_constants::pins::{I2cScl, I2cSda, JupiterI2c};
use crate::hal;
use crate::{app::*, Mono};

// Timestamp for logging
defmt::timestamp!("{=u64:us}", {
    Mono::now().duration_since_epoch().to_nanos()
});

pub fn startup(mut ctx: init::Context<'_>) -> (Shared, Local) {
    // Reset the spinlocks - this is skipped by soft-reset
    unsafe {
        hal::sio::spinlock_reset();
    }

    info!("Ejector startup");

    // Set up clocks
    let mut watchdog: Watchdog = Watchdog::new(ctx.device.WATCHDOG);
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

    let i2c0 = ctx.device.I2C0;

    // Set the pins to their default state
    let bank0_pins = hal::gpio::Pins::new(
        ctx.device.IO_BANK0,
        ctx.device.PADS_BANK0,
        sio.gpio_bank0,
        &mut ctx.device.RESETS,
    );

    

    

    // adc.free_running(&gegier_pin);
    // loop {
    //     adc.wait_ready();
    //     let reading = adc.read_single();
    //     if reading > 100 {
    //         info!("Reading: {}", reading as f32 * 3.3 / 4096.0);
    //     }
    // }



    let timer = hal::Timer::new_timer1(ctx.device.TIMER1, &mut ctx.device.RESETS, &clocks);
    let mut timer_two = timer;

    // Jupiter downlink UART

    let i2csda:I2cSda = bank0_pins.gpio24.reconfigure();
    let i2cscl:I2cScl = bank0_pins.gpio25.reconfigure();

 
    let jupiter_i2c = I2C::new_peripheral_event_iterator(i2c0, i2csda, i2cscl,  &mut ctx.device.RESETS,
        0x27 as u16);

 
    // SI1445 I2C
    // let guard_i2c: GuardI2C = I2C::i2c1(
    //     ctx.device.I2C1,
    //     bank0_pins.gpio26.reconfigure(),
    //     bank0_pins.gpio27.reconfigure(),
    //     100.kHz(),
    //     &mut ctx.device.RESETS,
    //     12.MHz(),
    // );

    info!("Peripherals initialized, spawning tasks");

    // Tasks

    (
        Shared {
            downlink_packets: Deque::new(),
            samples_buffer: [0u16; SAMPLE_COUNT],
        },
        Local {
            jupiter_i2c,
        
        },
    )
}
