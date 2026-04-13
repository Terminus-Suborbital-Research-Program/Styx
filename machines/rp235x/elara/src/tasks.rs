use bin_packets::data::adcs::AttitudeMetrics;
use bin_packets::devices::DeviceIdentifier;
use bin_packets::packets::status::Status;
use bin_packets::packets::ApplicationPacket;
use bincode::config::standard;
use bincode::encode_into_slice;

use bmi323::{AccelConfig, GyroConfig};
use bmm350::MagConfig;
use defmt::{error, info};
use embedded_hal::digital::{InputPin, StatefulOutputPin};

use crate::device_constants::{AvionicsI2cBus, MpChannel};
use crate::{app::*, device_constants::MotorI2cBus, Mono};
use embedded_hal::digital::OutputPin;
use embedded_io::Write;
use fugit::ExtU64;
use rtic::Mutex;
use rtic_monotonics::Monotonic;
use rtic_sync::arbiter::Arbiter;

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

pub async fn poll_attitude_metrics(mut ctx: poll_attitude_metrics::Context<'_>) {
    let mut rx_buf = [0u8; 128];
    let mut idx = 0;

    let config = bincode::config::standard();

    loop {
        // Read byte by byte if uart available
        while ctx.local.compute_link.uart_is_readable() {
            if idx >= rx_buf.len() {
                // Buffer full, break to avoid blocking
                break;
            }
            let mut byte = [0u8; 1];
            if let Ok(_) = ctx.local.compute_link.read_raw(&mut byte) {
                if idx < rx_buf.len() {
                    rx_buf[idx] = byte[0];
                    idx += 1;
                } else {
                    defmt::warn!("RX buffer overflow. Resetting to resync.");
                    idx = 0;
                    rx_buf[idx] = byte[0];
                    idx += 1;
                }
            } else {
                defmt::error!("UART read error");
                break;
            }
        }

        // If we've accumulated data, try to decode it
        if idx > 0 {
            match bincode::decode_from_slice::<AttitudeMetrics, _>(&rx_buf[..idx], config) {
                Ok((metrics, bytes_used)) => {
                    ctx.shared.metrics_buf.lock(|buf| {
                        if buf.is_full() {
                            let _ = buf.pop_front();
                        }
                        let _ = buf.push_back(metrics);
                    });

                    // Shift any remaining unparsed bytes to the front of the buffer
                    let remaining = idx - bytes_used;
                    if remaining > 0 {
                        rx_buf.copy_within(bytes_used..idx, 0);
                    }
                    idx = remaining;
                }
                Err(bincode::error::DecodeError::UnexpectedEnd { .. }) => {}
                Err(_) => {
                    // drop the oldest byte and shift
                    // the window by 1 to let bincode try again on the next loop.
                    rx_buf.copy_within(1..idx, 0);
                    idx -= 1;
                }
            }
        }

        Mono::delay(20.millis()).await;
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

    loop {
        let imu_result = ctx.local.bmi323.read_accel_data_scaled().await;
        match imu_result {
            Ok(acc) => {
                info!("Accel: {}, {}, {}", acc.x, acc.y, acc.z);
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
                info!("Gyro: {}, {}, {}", gyro.x, gyro.y, gyro.z);
                let gyro_packet = ApplicationPacket::GyroscopeData {
                    timestamp: now_timestamp().millis(),
                    x: gyro.x,
                    y: gyro.y,
                    z: gyro.z,
                };
                ctx.shared.data.lock(|data| {
                    data.push_back(gyro_packet).ok();
                });
            }
            Err(i2c_error) => {
                error!("BMI: {}", i2c_error);
            }
        }
        let mag_result = ctx.local.bmm350.read_mag_data_scaled().await;
        match mag_result {
            Ok(mag) => {
                info!("Mag: {}, {}, {}", mag.x, mag.y, mag.z);
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
        });
        Mono::delay(100.millis()).await;
    }
}

pub async fn inertial_nav(_ctx: inertial_nav::Context<'_>) {
    loop {
        info!("Inertial Navigation");
        Mono::delay(250_u64.millis()).await;
    }
}

pub async fn read_photodiode(mut ctx: read_photodiode::Context<'_>) {
    let mut i: usize = 0;

    loop {
        i = i + 1;

        if (i % 4) == 0 {
            match ctx.local.mp_channel {
                MpChannel::PD1_4 => {
                    ctx.local.pin19.set_low().unwrap();
                    ctx.local.pin20.set_low().unwrap();
                    ctx.local.pin21.set_low().unwrap();

                    *ctx.local.mp_channel = MpChannel::PD5_8;
                }

                MpChannel::PD5_8 => {
                    ctx.local.pin19.set_high().unwrap();
                    ctx.local.pin20.set_low().unwrap();
                    ctx.local.pin21.set_low().unwrap();

                    *ctx.local.mp_channel = MpChannel::PD9_12;
                }

                MpChannel::PD9_12 => {
                    ctx.local.pin19.set_low().unwrap();
                    ctx.local.pin20.set_high().unwrap();
                    ctx.local.pin21.set_low().unwrap();

                    *ctx.local.mp_channel = MpChannel::PD13_16;
                }

                MpChannel::PD13_16 => {
                    ctx.local.pin19.set_high().unwrap();
                    ctx.local.pin20.set_high().unwrap();
                    ctx.local.pin21.set_low().unwrap();

                    *ctx.local.mp_channel = MpChannel::PD17_20;
                }

                MpChannel::PD17_20 => {
                    ctx.local.pin19.set_low().unwrap();
                    ctx.local.pin20.set_low().unwrap();
                    ctx.local.pin21.set_high().unwrap();

                    *ctx.local.mp_channel = MpChannel::PD21_24;
                }

                MpChannel::PD21_24 => {
                    ctx.local.pin19.set_high().unwrap();
                    ctx.local.pin20.set_low().unwrap();
                    ctx.local.pin21.set_high().unwrap();

                    *ctx.local.mp_channel = MpChannel::PD25_28;
                }

                MpChannel::PD25_28 => {
                    ctx.local.pin19.set_low().unwrap();
                    ctx.local.pin20.set_high().unwrap();
                    ctx.local.pin21.set_high().unwrap();

                    *ctx.local.mp_channel = MpChannel::PD29_32;
                }

                MpChannel::PD29_32 => {
                    ctx.local.pin19.set_high().unwrap();
                    ctx.local.pin20.set_high().unwrap();
                    ctx.local.pin21.set_high().unwrap();

                    *ctx.local.mp_channel = MpChannel::PD1_4;
                }
            }
        }
        ctx.local.adc_outputs[i] = ctx.local.adc_fifo_l.as_mut().unwrap().read().unwrap();
        info!("Added {} to adc_outputs.", ctx.local.adc_outputs[i]);

        i = i % 23;
        Mono::delay(30_u64.micros()).await;
    }
}
