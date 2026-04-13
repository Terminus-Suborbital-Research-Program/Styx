#![no_std]
#![no_main]

mod peripherals;

use core::mem::MaybeUninit;

use bmi323::{
    AccelConfig, AccelerometerPowerMode, AccelerometerRange, AsyncBmi323, AsyncI2cInterface,
    AverageNum, GyroConfig, GyroscopePowerMode, GyroscopeRange, OutputDataRate,
};
use defmt_rtt as _;
use rp235x_pac::interrupt;

#[cfg(feature = "rp2350")]
use rp235x_hal as hal;

#[cfg(feature = "rp2350")]
use rtic_monotonics::systick::prelude::*;
#[cfg(feature = "rp2350")]
systick_monotonic!(Mono, 1_000_000);

#[cfg(feature = "rp2350")]
mod rtic_device {
    pub use rp235x_pac::*;

    pub mod interrupt {
        pub use rp235x_pac::Interrupt::*;
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    defmt::error!("Panic: {}", info);
    hal::halt();
}

#[link_section = ".start_block"]
#[used]
#[cfg(feature = "rp2350")]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    hal::binary_info::rp_cargo_bin_name!(),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"Async RTIC BMI323 example"),
    hal::binary_info::rp_cargo_homepage_url!(),
    hal::binary_info::rp_program_build_attribute!(),
];

#[rtic::app(
    device = crate::rtic_device,
    dispatchers = [PIO2_IRQ_0, PIO2_IRQ_1, DMA_IRQ_0],
    peripherals = true,
)]
mod app {
    use super::*;
    use crate::peripherals::async_i2c::AsyncI2c;

    use embedded_hal::digital::StatefulOutputPin;
    use fugit::{ExtU64, RateExtU32};
    use hal::{
        clocks,
        gpio::{
            self, bank0::Gpio4, bank0::Gpio5, FunctionI2C, FunctionSio, Pin, PullNone, PullUp,
            SioOutput,
        },
        Clock, Sio, Watchdog, I2C,
    };
    use rtic_sync::arbiter::{i2c::ArbiterDevice, Arbiter};

    pub const XTAL_FREQ_HZ: u32 = 12_000_000;

    pub type AvionicsI2cBus = AsyncI2c<
        I2C<
            hal::pac::I2C0,
            (
                Pin<Gpio4, FunctionI2C, PullUp>,
                Pin<Gpio5, FunctionI2C, PullUp>,
            ),
        >,
    >;

    #[shared]
    pub struct Shared {}

    #[local]
    pub struct Local {
        pub led: gpio::Pin<gpio::bank0::Gpio25, FunctionSio<SioOutput>, PullNone>,
        pub bmi323: AsyncBmi323<AsyncI2cInterface<ArbiterDevice<'static, AvionicsI2cBus>>, Mono>,
    }

    #[init(local = [i2c_bus: MaybeUninit<Arbiter<AvionicsI2cBus>> = MaybeUninit::uninit()])]
    fn init(mut ctx: init::Context) -> (Shared, Local) {
        let mut watchdog = Watchdog::new(ctx.device.WATCHDOG);

        let sio = Sio::new(ctx.device.SIO);
        let pins = hal::gpio::Pins::new(
            ctx.device.IO_BANK0,
            ctx.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut ctx.device.RESETS,
        );

        let clocks = clocks::init_clocks_and_plls(
            XTAL_FREQ_HZ,
            ctx.device.XOSC,
            ctx.device.CLOCKS,
            ctx.device.PLL_SYS,
            ctx.device.PLL_USB,
            &mut ctx.device.RESETS,
            &mut watchdog,
        )
        .unwrap();

        Mono::start(ctx.core.SYST, clocks.system_clock.freq().to_Hz());

        let led = pins
            .gpio25
            .into_pull_type::<PullNone>()
            .into_push_pull_output();

        let sda_pin: Pin<Gpio4, FunctionI2C, PullUp> = pins.gpio4.reconfigure();
        let scl_pin: Pin<Gpio5, FunctionI2C, PullUp> = pins.gpio5.reconfigure();

        let i2c = I2C::i2c0(
            ctx.device.I2C0,
            sda_pin,
            scl_pin,
            400.kHz(),
            &mut ctx.device.RESETS,
            &clocks.system_clock,
        );

        let async_i2c = AsyncI2c::new(i2c, 100000);
        let i2c_bus = ctx.local.i2c_bus.write(Arbiter::new(async_i2c));
        let bmi323 = AsyncBmi323::new_with_i2c(ArbiterDevice::new(i2c_bus), 0x68, Mono);

        heartbeat::spawn().ok();
        sample_imu::spawn().ok();

        defmt::info!("async-simple initialized");

        (Shared {}, Local { led, bmi323 })
    }

    #[task(local = [led], priority = 1)]
    async fn heartbeat(ctx: heartbeat::Context) {
        loop {
            let _ = ctx.local.led.toggle();
            Mono::delay(500_u64.millis()).await;
        }
    }

    #[task(local = [bmi323], priority = 2)]
    async fn sample_imu(ctx: sample_imu::Context) {
        defmt::info!("Initializing BMI323");

        if let Err(err) = ctx.local.bmi323.init().await {
            defmt::error!("BMI323 init failed: {}", err);
            loop {
                Mono::delay(1000_u64.millis()).await;
            }
        }

        let accel_config = AccelConfig::builder()
            .mode(AccelerometerPowerMode::HighPerf)
            .range(AccelerometerRange::G8)
            .odr(OutputDataRate::Odr100hz)
            .avg_num(AverageNum::Avg8)
            .build();
        if let Err(err) = ctx.local.bmi323.set_accel_config(accel_config).await {
            defmt::error!("BMI323 accel config failed: {}", err);
        }

        let gyro_config = GyroConfig::builder()
            .mode(GyroscopePowerMode::HighPerf)
            .range(GyroscopeRange::DPS2000)
            .odr(OutputDataRate::Odr100hz)
            .avg_num(AverageNum::Avg8)
            .build();
        if let Err(err) = ctx.local.bmi323.set_gyro_config(gyro_config).await {
            defmt::error!("BMI323 gyro config failed: {}", err);
        }

        defmt::info!("BMI323 configured");

        loop {
            match ctx.local.bmi323.read_accel_data_scaled().await {
                Ok(accel) => defmt::info!(
                    "accel m/s^2 => x: {}, y: {}, z: {}",
                    accel.x,
                    accel.y,
                    accel.z
                ),
                Err(err) => defmt::error!("BMI323 accel read failed: {}", err),
            }

            match ctx.local.bmi323.read_gyro_data_scaled().await {
                Ok(gyro) => defmt::info!("gyro dps => x: {}, y: {}, z: {}", gyro.x, gyro.y, gyro.z),
                Err(err) => defmt::error!("BMI323 gyro read failed: {}", err),
            }

            Mono::delay(250_u64.millis()).await;
        }
    }
}

#[interrupt]
unsafe fn I2C0_IRQ() {
    app::AvionicsI2cBus::on_interrupt();
}
