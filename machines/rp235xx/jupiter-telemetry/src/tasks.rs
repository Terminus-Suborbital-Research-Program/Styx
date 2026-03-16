use bin_packets::devices::DeviceIdentifier;
use bin_packets::packets::status::Status;
use bin_packets::packets::ApplicationPacket;
use bincode::config::standard;
use bincode::{Encode, encode_into_slice};

use bmi323::{AccelConfig, GyroConfig};
use bmm350::MagConfig;
use defmt::{error, info};
use embedded_hal::digital::{InputPin, StatefulOutputPin};
use rp235x_hal::async_utils::AsyncPeripheral;

use crate::device_constants::AvionicsI2cBus;
use crate::{app::*, device_constants::ComputeI2cBus, Mono};
use embedded_io::Write;
use fugit::ExtU64;
use rtic::Mutex;
use rtic_monotonics::Monotonic;
use rtic_sync::arbiter::Arbiter;

use rp235x_hal::i2c::peripheral::Event;

use bmp5::Measurement;

pub async fn heartbeat(mut ctx: heartbeat::Context<'_>) {
    let mut sequence_number: u16 = 0;
    loop {
        _ = ctx.local.led.toggle();

        let status = Status::new(DeviceIdentifier::Icarus, now_timestamp(), sequence_number);

        ctx.shared
            .data
            .lock(|data| data.push_back(status.into()).ok());

        sequence_number = sequence_number.wrapping_add(1);

        Mono::delay(300_u64.millis()).await;
    }
}


pub async fn sample_sensors(
    mut ctx: sample_sensors::Context<'_>,
    _avionics_i2c: &'static Arbiter<AvionicsI2cBus>,
) {
    ctx.local.bme280.init().await.ok();
    ctx.local.bmi323.init().await.ok();
    let accel_config = AccelConfig::builder()
        .mode(bmi323::AccelerometerPowerMode::HighPerf)
        .range(bmi323::AccelerometerRange::G8)
        .odr(bmi323::OutputDataRate::Odr100hz)
        .avg_num(bmi323::AverageNum::Avg8);
    ctx.local
        .bmi323
        .set_accel_config(accel_config.build())
        .await
        .ok();
    let gyro_config = GyroConfig::builder()
        .mode(bmi323::GyroscopePowerMode::HighPerf)
        .range(bmi323::GyroscopeRange::DPS125)
        .odr(bmi323::OutputDataRate::Odr100hz)
        .avg_num(bmi323::AverageNum::Avg8);

    ctx.local
        .bmi323
        .set_gyro_config(gyro_config.build())
        .await
        .ok();

    ctx.local.bmm350.init().await.ok();
    let mag_config = MagConfig::builder().performance(bmm350::PerformanceMode::Regular);
    ctx.local
        .bmm350
        .set_power_mode(bmm350::PowerMode::Normal)
        .await
        .ok();
    ctx.local
        .bmm350
        .set_mag_config(mag_config.build())
        .await
        .ok();

    ctx.local.bmp5.init().await.unwrap();

    loop {
        ctx.shared.data.lock(|data| {
                    data.clear();
                });
        let imu_result = ctx.local.bmi323.read_accel_data_scaled().await;
        match imu_result {
            Ok(acc) => {
                // info!("Accel: {}, {}, {}", acc.x, acc.y, acc.z);
                let acceleration_packet = ApplicationPacket::AccelerometerData {
                    timestamp: now_timestamp().millis(),
                    x: acc.x,
                    y: acc.y,
                    z: acc.z,
                };
                ctx.shared.data.lock(|data| {
                    data.push_back(acceleration_packet).ok();
                });
            }
            Err(i2c_error) => {
                error!("BMI: {}", i2c_error);
            }
        }
        let gyro_result = ctx.local.bmi323.read_gyro_data_scaled().await;
        match gyro_result {
            Ok(gyro) => {
                // info!("Gyro: {}, {}, {}", gyro.x, gyro.y, gyro.z);
                let gyro_packet = ApplicationPacket::GyroscopeData {
                    timestamp: now_timestamp().millis(),
                    x: gyro.x,
                    y: gyro.y,
                    z: gyro.z,
                };
                ctx.shared.data.lock(|data| {
                    data.push_back(gyro_packet).ok();
                    // data. gyro_packet;
                });
            }
            Err(i2c_error) => {
                error!("BMI: {}", i2c_error);
            }
        }
        let mag_result = ctx.local.bmm350.read_mag_data_scaled().await;
        match mag_result {
            Ok(mag) => {
                // info!("Mag: {}, {}, {}", mag.x, mag.y, mag.z);
                let mag_packet = ApplicationPacket::MagnetometerData {
                    timestamp: now_timestamp().millis(),
                    x: mag.x,
                    y: mag.y,
                    z: mag.z,
                };
                ctx.shared.data.lock(|data| {
                    data.push_back(mag_packet).ok();

                });
            }
            Err(i2c_error) => {
                error!("BMM: {}", i2c_error);
            }
        }
        let env = ctx.local.bme280.sample().await;
        let env_packet = ApplicationPacket::EnvironmentData {
            timestamp: now_timestamp().millis(),
            temperature: env.1,
            pressure: env.2,
            humidity: env.3,
        };
         ctx.shared.data.lock(|data| {
            data.push_back(env_packet).ok();
            // data[3] = env_packet;
        });
        let bmp5_dat = ctx.local.bmp5.measure().await.unwrap();

        let bmp_5_packet = ApplicationPacket::BMPData { 
            timestamp: now_timestamp().millis(),
            temperature: bmp5_dat.temperature,
            pressure: bmp5_dat.pressure,
        };

         ctx.shared.data.lock(|data| {
            data.push_back(env_packet).ok();
        });

        // info!("BMP 5 temp: {:?}", );
        // info!("BMP 5 press: {:?}", );

        // info!("Bytes: {:?}", bytes);

        Mono::delay(100.millis()).await;
    }
}

use rp235x_pac::interrupt;
#[interrupt]
unsafe fn I2C0_IRQ() {
    ComputeI2cBus::on_interrupt();
}
pub async fn get_data_response( mut ctx: get_data_response::Context<'_>) {
    let mut outgoing_buf = [0u8; 512];
    let mut buf_len = 0;

    let mut read_pos = 0; // Current outgoing packet byte 
    let mut write_pos = 0; // Location in serialization

    loop {
        let event = ctx.local.compute_i2c.wait_next().await;
        match event {
            Event::Start => {
                read_pos = 0;
                write_pos = 0;
                
                ctx.shared.data.lock(|data| {
                    while let Some(packet) = data.pop_front() {
                        if let Ok(w) = encode_into_slice(packet, &mut outgoing_buf[write_pos..], standard()) {
                            write_pos += w;
                        } else {
                            data.push_front(packet).ok();
                            break;
                        }
                    }
                });
            }

            Event::TransferRead => {
                if read_pos < write_pos {
                    // Send the next byte to the controller 
                    ctx.local.compute_i2c.write(&[outgoing_buf[read_pos]]);
                    read_pos += 1;
                } else {
                    // Send padding byte
                    ctx.local.compute_i2c.write(&[0x00]);
                }
            }

            // There are other events than transfer read, but this use case is so simple
            // that I think I can just throw them, but we'll see with testing
            _ => {

            }
        }
    }
}

// Sample Functions
use crate::peripherals::async_i2c::AsyncI2c;
use ina260_terminus::AsyncINA260;
use rp235x_hal::{
    gpio::{Pin, PullUp},
    I2C,
};
use rtic_sync::arbiter::i2c::ArbiterDevice;