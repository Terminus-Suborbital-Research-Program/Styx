use bincode::de;
use bincode::{config::standard, error::DecodeError};
use bme280_rs::{AsyncBme280, Configuration, Oversampling, SensorMode};
use defmt::{error, info, trace};
use embedded_hal::digital::StatefulOutputPin;
use embedded_hal_async::i2c::{self, I2c};
use embedded_hal_bus::{i2c::AtomicDevice, util::AtomicCell};
use embedded_io::Read;
use fugit::ExtU64;
use futures::FutureExt as _;
use ina260_terminus::{AsyncINA260, Register as INA260Register};
use mcf8316c_rs::{controller::MotorController, data_word_to_u32, registers::write_sequence};
use rtic::Mutex;
use rtic_monotonics::Monotonic;
use rtic_sync::arbiter::{i2c::ArbiterDevice, Arbiter};

use crate::device_constants::AvionicsI2cBus;
use crate::phases::StateMachineListener;
use crate::{
    app::{incoming_packet_handler, *},
    communications::{
        hc12::BaudRate,
        link_layer::{LinkLayerPayload, LinkPacket},
    },
    device_constants::MotorI2cBus,
    peripherals::async_i2c::{self, AsyncI2cError},
    Mono,
};

pub async fn heartbeat(ctx: heartbeat::Context<'_>) {
    loop {
        _ = ctx.local.led.toggle();

        Mono::delay(300_u64.millis()).await;
    }
}

pub fn uart_interrupt(mut ctx: uart_interrupt::Context<'_>) {
    ctx.shared.radio_link.lock(|radio| {
        radio.device.update().ok();
    });
}

use rp235x_pac::interrupt;
#[interrupt]
unsafe fn I2C0_IRQ() {
    MotorI2cBus::on_interrupt();
}

pub async fn motor_drivers(
    mut ctx: motor_drivers::Context<'_>,
    i2c: &'static Arbiter<MotorI2cBus>,
    mut esc_state_listener: StateMachineListener,
) {
    esc_state_listener
        .wait_for_state_specific(bin_packets::IcarusPhase::OrientSolar)
        .await;
    info!("Motor Driver Task Started");

    ctx.local.ina260_1.init().await.ok();
    ctx.local.ina260_2.init().await.ok();
    ctx.local.ina260_3.init().await.ok();

    loop {
        ctx.local
            .ina260_1
            .read_register(INA260Register::VOLTAGE)
            .await
            .ok();
        ctx.local
            .ina260_2
            .read_register(INA260Register::VOLTAGE)
            .await
            .ok();
        ctx.local
            .ina260_3
            .read_register(INA260Register::VOLTAGE)
            .await
            .ok();
        Mono::delay(100_u64.millis()).await;
    }
}

pub async fn radio_flush(mut ctx: radio_flush::Context<'_>) {
    let mut on_board_baudrate: BaudRate = BaudRate::B9600;
    let bytes_to_flush = 16;

    loop {
        ctx.shared.radio_link.lock(|radio| {
            radio.device.flush(bytes_to_flush).ok();
            on_board_baudrate = radio.device.get_baudrate();
        });

        // Need to wait wait the in-air baudrate, or the on-board baudrate
        // whichever is slower

        let mut slower =
            core::cmp::min(on_board_baudrate.to_u32(), on_board_baudrate.to_in_air_bd());

        // slower is bps, so /1000 to get ms
        slower /= 1000;

        // Delay for that times the number of bytes flushed
        Mono::delay((slower as u64 * bytes_to_flush as u64).millis()).await;
    }
}

pub async fn incoming_packet_handler(mut ctx: incoming_packet_handler::Context<'_>) {
    loop {
        let buffer = ctx
            .shared
            .radio_link
            .lock(|radio| radio.device.clone_buffer());

        let decode: Result<(LinkPacket, usize), bincode::error::DecodeError> =
            bincode::decode_from_slice(&buffer, standard());

        match decode {
            Err(e) => match e {
                #[allow(unused_variables)]
                DecodeError::UnexpectedVariant {
                    type_name,
                    allowed,
                    found,
                } => {
                    // Clear the buffer
                    ctx.shared.radio_link.lock(|radio| radio.device.clear());
                }

                #[allow(unused_variables)]
                DecodeError::UnexpectedEnd { additional } => {
                    // Nothing to do here
                }

                // These decoding errors cause us to pop the front of the buffer to remove the character
                #[allow(unused_variables)]
                DecodeError::InvalidIntegerType { expected, found } => {
                    let mut buffer = [0u8; 1];
                    ctx.shared
                        .radio_link
                        .lock(|radio| radio.device.read(&mut buffer).ok());
                }

                _ => {
                    // Something else, log it. Bincode doen't impliment defmt::Debug unfortunately
                    error!("Decoding error: {:?}", defmt::Debug2Format(&e));
                }
            },

            Ok(packet_wrapper) => {
                let packet = packet_wrapper.0;
                let read = packet_wrapper.1;
                // Drop the read bytes
                ctx.shared
                    .radio_link
                    .lock(|radio| radio.device.drop_bytes(read));

                // Uncomment the below if you think you made a mistake in handling

                // let mut buffer_heapless_stirng: alloc::string::String =
                //     alloc::string::String::new();
                // write!(buffer_heapless_stirng, "{:#?}", packet).ok();
                // for char in buffer_heapless_stirng.chars() {
                //     print!(ctx, "{}", char);
                //     Mono::delay(1_u64.millis()).await;
                // }
                // println!(ctx, "\n");

                // Check the checksum, if it fails, the packet is bad, we should continue
                // and clear the buffer
                // if !packet.verify_checksum() {
                //     ctx.shared.radio_link.lock(|radio| radio.device.clear());
                //     continue;
                // }

                // if let LinkLayerPayload::Payload(app_packet) = packet.payload {
                //     match app_packet {
                //         bin_packets::ApplicationPacket::Command(command) => match command {
                //             _ => {}
                //         },

                //         _ => {
                //             let mut buffer_heapless_stirng: alloc::string::String =
                //                 alloc::string::String::new();
                //             write!(buffer_heapless_stirng, "{:#?}", packet).ok();
                //         }
                //     }
                // }
            }
        }

        Mono::delay(10_u64.millis()).await;
    }
}

pub async fn sample_sensors(
    mut ctx: sample_sensors::Context<'_>,
    avionics_i2c: &'static Arbiter<AvionicsI2cBus>,
) {
    ctx.local.bme280.init().await.ok();
    ctx.local
        .bme280
        .set_sampling_configuration(
            Configuration::default()
                .with_temperature_oversampling(Oversampling::Oversample1)
                .with_pressure_oversampling(Oversampling::Oversample1)
                .with_humidity_oversampling(Oversampling::Oversample1)
                .with_sensor_mode(SensorMode::Normal),
        )
        .await
        .expect("Failed to configure BME280");

    Mono::delay(10_u64.millis()).await; // !TODO (Remove me if no effect) Delaying preemptive to other processes just in case...

    // let avionics_arbiter = ctx.local.i2c_avionics_bus.read(Arbiter::new(avionics_i2c));
    // let mut bme280 = BME280::new_primary(ArbiterDevice::new(avionics_arbiter));

    // ctx.shared.software_delay.lock(|mut delay: &mut rp235x_hal::Timer<rp235x_hal::timer::CopyableTimer1>|{
    //     bme280.init(delay);
    // });

    loop {
        if let Ok(Some(temperature)) = ctx.local.bme280.read_temperature().await {
            info!("Temperature: {}", temperature);
        }
        if let Ok(Some(pressure)) = ctx.local.bme280.read_pressure().await {
            info!("Pressure: {}", pressure);
        }
        if let Ok(Some(humidity)) = ctx.local.bme280.read_humidity().await {
            info!("Humidity: {}", humidity);
        }
        Mono::delay(250_u64.millis()).await;
    }
}

pub async fn inertial_nav(_ctx: inertial_nav::Context<'_>) {
    loop {
        // info!("Inertial Navigation");
        Mono::delay(250_u64.millis()).await;
    }
}
