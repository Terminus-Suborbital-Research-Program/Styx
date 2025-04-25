use bincode::de;
use bincode::{config::standard, error::DecodeError};
use bme280_rs::{AsyncBme280, Configuration, Oversampling, SensorMode};
use bmi323::BMI323Status;
use defmt::{dbg, error, info, trace};
use embedded_hal::digital::StatefulOutputPin;
use embedded_hal_async::i2c::{self, I2c};
use embedded_hal_bus::{i2c::AtomicDevice, util::AtomicCell};
use embedded_io::Read;
use fugit::ExtU64;
use futures::FutureExt as _;
use ina260_terminus::{AsyncINA260, Register as INA260Register};
use mcf8316c_rs::{controller::MotorController, data_word_to_u32, registers::write_sequence};
use rp235x_pac::sysinfo::chip_id;
use rtic::Mutex;
use rtic_monotonics::Monotonic;
use rtic_sync::arbiter::{i2c::ArbiterDevice, Arbiter};

use crate::device_constants::AvionicsI2cBus;
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
unsafe fn I2C1_IRQ() {
    AvionicsI2cBus::on_interrupt();
}

pub async fn motor_drivers(
    ctx: motor_drivers::Context<'_>,
    motor_i2c: &'static Arbiter<MotorI2cBus>,
) {
    info!("Motor Driver Task Started");
    let ina260_1_init = ctx.local.ina260_1.init().await;
    match ina260_1_init {
        Ok(_) => {
            info!("INA260 1 Initialized");
        }
        Err(e) => {
            error!("INA260 1 Initialization Error: {:?}", e);
        }
    }
    let ina260_2_init = ctx.local.ina260_2.init().await;
    match ina260_2_init {
        Ok(_) => {
            info!("INA260 2 Initialized");
        }
        Err(e) => {
            error!("INA260 2 Initialization Error: {:?}", e);
        }
    }

    let ina260_3_init = ctx.local.ina260_3.init().await;
    match ina260_3_init {
        Ok(_) => {
            info!("INA260 3 Initialized");
        }
        Err(e) => {
            error!("INA260 3 Initialization Error: {:?}", e);
        }
    }
    loop {
        let chip_id_1 = ctx.local.ina260_1.rid().await.ok();
        let voltage_1 = ctx.local.ina260_1.voltage().await.ok();
        let current_1 = ctx.local.ina260_1.current().await.ok();
        let power_1 = ctx.local.ina260_1.power().await.ok();
        info!("Chip ID 1: {}", chip_id_1);
        info!("V: {}", voltage_1);
        info!("I: {}", current_1);
        info!("P: {}", power_1);

        let chip_id_2 = ctx.local.ina260_2.rid().await;
        let voltage_2 = ctx.local.ina260_2.voltage().await.ok();
        let current_2 = ctx.local.ina260_2.current().await.ok();
        let power_2 = ctx.local.ina260_2.power().await.ok();
        info!(
            "Chip ID 2: {}, {}, {}, {}",
            chip_id_2, voltage_2, current_2, power_2
        );
        let chip_id_3 = ctx.local.ina260_2.rid().await;
        let voltage_3 = ctx.local.ina260_3.voltage().await.ok();
        let current_3 = ctx.local.ina260_3.current().await.ok();
        let power_3 = ctx.local.ina260_3.power().await.ok();
        info!(
            "Chip ID 3: {}, {}, {}, {}",
            chip_id_3, voltage_3, current_3, power_3
        );
        Mono::delay(500_u64.millis()).await;
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
    avionics_i2c: &'static Arbiter<MotorI2cBus>,
) -> ! {
    let intialize_status = ctx.local.bme280.init().await;
    info!("BME Initialize Status: {}", intialize_status);
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

    let result = ctx.local.bme280.chip_id().await;
    info!("Result: {}", result);

    Mono::delay(50_u64.millis()).await; // !TODO (Remove me if no effect) Delaying preemptive to other processes just in case...

    // let mut buf: [u8; 2] = [0; 2];
    loop {
        //         let mut i2c_line = avionics_i2c.access().await;
        // let imu_result = i2c_line.write_read(0x68, &[0x01], &mut buf).await;
        // match imu_result {
        //     Ok(_) => {
        //         info!("Chip Id: {}", buf[0]);
        //         info!("Chip Id: {}", buf[1]);
        //         trace!("What's up");
        //     }
        //     Err(e) => {
        //         error!("Error reading IMU: {:?}", e);
        //     }
        // }

        // let result = ctx.local.bmi323.check_init_status().await;
        // match result {
        //     Ok(BMI323Status::INIT_OK) => {
        //         info!("BMI323 is initialized");
        //     }
        //     Err(e) => {
        //         error!("Error checking BMI323 init status: {:?}", e);
        //     }
        //     _ => {
        //         info!("BMI323 is not initialized");
        //     }
        // }

        // let result = ctx.local.bme280.chip_id().await;
        // info!("Result: {}", result);
        info!("Sample Sensors Running");
        if let Ok(Some(temperature)) = ctx.local.bme280.read_temperature().await {
            info!("Temperature: {}", temperature);
        }
        if let Ok(Some(pressure)) = ctx.local.bme280.read_pressure().await {
            info!("Pressure: {}", pressure);
        }
        if let Ok(Some(humidity)) = ctx.local.bme280.read_humidity().await {
            info!("Humidity: {}", humidity);
        }
        Mono::delay(100_u64.millis()).await;
    }
}

pub async fn inertial_nav(_ctx: inertial_nav::Context<'_>) {
    loop {
        // info!("Inertial Navigation");
        Mono::delay(250_u64.millis()).await;
    }
}
