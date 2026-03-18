use bin_packets::devices::DeviceIdentifier;
use bin_packets::packets::status::Status;
use bin_packets::packets::ApplicationPacket;
use bincode::config::standard;
use bincode::encode_into_slice;

use bmi323::{AccelConfig, GyroConfig};
use bmm350::MagConfig;
use defmt::{error, info};
use embedded_hal::digital::{InputPin, StatefulOutputPin};

use crate::device_constants::{
    AvionicsI2cBus,
    MpChannel,
};
use crate::{app::*, device_constants::MotorI2cBus, Mono};
use embedded_io::Write;
use fugit::ExtU64;
use rtic::Mutex;
use rtic_monotonics::Monotonic;
use rtic_sync::arbiter::Arbiter;
use embedded_hal::digital::OutputPin;

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

// use rp235x_pac::interrupt;
// #[interrupt]
// unsafe fn I2C0_IRQ() {
//     MotorI2cBus::on_interrupt();
// }



// pub async fn ina_sample(mut ctx: ina_sample::Context<'_>, _i2c: &'static Arbiter<MotorI2cBus>) {
//     info!("INA Sample Task Started");
//     if let Err(e) = ctx.local.ina260_1.init().await {
//         error!("Error initializing INA 1: {:?}", e);
//     }
//     Mono::delay(2_u64.millis()).await;

//     if let Err(e) = ctx.local.ina260_2.init().await {
//         error!("Error initializing INA 2: {:?}", e);
//     }
//     Mono::delay(2_u64.millis()).await;

//     if let Err(e) = ctx.local.ina260_3.init().await {
//         error!("Error initializing INA 3: {:?}", e);
//     }
//     Mono::delay(2_u64.millis()).await;
//     if let Err(e) = ctx.local.ina260_4.init().await {
//         error!("Error initializing INA 4: {:?}", e);
//     }
//     Mono::delay(2_u64.millis()).await;

//     ctx.local
//         .ina260_1
//         .set_operating_mode(ina260_terminus::OperMode::SCBVC)
//         .await
//         .ok();
//     ctx.local
//         .ina260_2
//         .set_operating_mode(ina260_terminus::OperMode::SCBVC)
//         .await
//         .ok();
//     ctx.local
//         .ina260_3
//         .set_operating_mode(ina260_terminus::OperMode::SCBVC)
//         .await
//         .ok();
//     ctx.local
//         .ina260_4
//         .set_operating_mode(ina260_terminus::OperMode::SCBVC)
//         .await
//         .ok();

//     loop {
//         let ina_samples = ina_data_handle(
//             ctx.local.ina260_1,
//             ctx.local.ina260_2,
//             ctx.local.ina260_3,
//             ctx.local.ina260_4,
//         )
//         .await;
//         ctx.shared.data.lock(|data| {
//             let voltages_packet = ApplicationPacket::VoltageData {
//                 timestamp: ina_samples.0.0,
//                 voltage: ina_samples.1.0,
//             };
//             let current_packet = ApplicationPacket::CurrentData {
//                 timestamp: ina_samples.0.1,
//                 current: ina_samples.1.1,
//             };
//             let power_packet = ApplicationPacket::PowerData {
//                 timestamp: ina_samples.0.2,
//                 power: ina_samples.1.2,
//             };
//             info!("Voltage Packet: {}", voltages_packet);
//             info!("Current Packet: {}", current_packet);
//             info!("Power Packet: {}", power_packet);
//             data.push_back(voltages_packet).ok();
//             data.push_back(current_packet).ok();
//             data.push_back(power_packet).ok();
//         });
//         Mono::delay(250.millis()).await;
//     }
// }

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

// Sample Functions
use crate::peripherals::async_i2c::AsyncI2c;
use ina260_terminus::AsyncINA260;
use rp235x_hal::{
    gpio::{Pin, PullUp},
    I2C,
};
use rtic_sync::arbiter::i2c::ArbiterDevice;
// #[allow(clippy::type_complexity)]
// async fn ina_data_handle(
//     ina260_1: &mut AsyncINA260<
//         ArbiterDevice<
//             '_,
//             AsyncI2c<
//                 I2C<
//                     rp235x_pac::I2C0,
//                     (
//                         Pin<rp235x_hal::gpio::bank0::Gpio16, rp235x_hal::gpio::FunctionI2c, PullUp>,
//                         Pin<rp235x_hal::gpio::bank0::Gpio17, rp235x_hal::gpio::FunctionI2c, PullUp>,
//                     ),
//                 >,
//             >,
//         >,
//         Mono,
//     >,
//     ina260_2: &mut AsyncINA260<
//         ArbiterDevice<
//             '_,
//             AsyncI2c<
//                 I2C<
//                     rp235x_pac::I2C0,
//                     (
//                         Pin<rp235x_hal::gpio::bank0::Gpio16, rp235x_hal::gpio::FunctionI2c, PullUp>,
//                         Pin<rp235x_hal::gpio::bank0::Gpio17, rp235x_hal::gpio::FunctionI2c, PullUp>,
//                     ),
//                 >,
//             >,
//         >,
//         Mono,
//     >,
//     ina260_3: &mut AsyncINA260<
//         ArbiterDevice<
//             '_,
//             AsyncI2c<
//                 I2C<
//                     rp235x_pac::I2C0,
//                     (
//                         Pin<rp235x_hal::gpio::bank0::Gpio16, rp235x_hal::gpio::FunctionI2c, PullUp>,
//                         Pin<rp235x_hal::gpio::bank0::Gpio17, rp235x_hal::gpio::FunctionI2c, PullUp>,
//                     ),
//                 >,
//             >,
//         >,
//         Mono,
//     >,
//     ina260_4: &mut AsyncINA260<
//         ArbiterDevice<
//             '_,
//             AsyncI2c<
//                 I2C<
//                     rp235x_pac::I2C0,
//                     (
//                         Pin<rp235x_hal::gpio::bank0::Gpio16, rp235x_hal::gpio::FunctionI2c, PullUp>,
//                         Pin<rp235x_hal::gpio::bank0::Gpio17, rp235x_hal::gpio::FunctionI2c, PullUp>,
//                     ),
//                 >,
//             >,
//         >,
//         Mono,
//     >,
// ) -> (
//     ([u64; 4], [u64; 4], [u64; 4]),
//     ([f32; 4], [f32; 4], [f32; 4]),
// ) {
//     let voltage_1 = ina260_1.voltage().await;
//     let v1_ts = now_timestamp().millis();

//     let voltage_2 = ina260_2.voltage().await;
//     let v2_ts = now_timestamp().millis();

//     let voltage_3 = ina260_3.voltage().await;
//     let v3_ts = now_timestamp().millis();

//     let voltage_4 = ina260_4.voltage().await;
//     let v4_ts = now_timestamp().millis();

//     let current_1 = ina260_1.current().await;
//     let i1_ts = now_timestamp().millis();

//     let current_2 = ina260_2.current().await;
//     let i2_ts = now_timestamp().millis();

//     let current_3 = ina260_3.current().await;
//     let i3_ts = now_timestamp().millis();

//     let current_4 = ina260_4.current().await;
//     let i4_ts = now_timestamp().millis();

//     let power_1 = ina260_1.power().await;
//     let p1_ts = now_timestamp().millis();

//     let power_2 = ina260_2.power().await;
//     let p2_ts = now_timestamp().millis();

//     let power_3 = ina260_3.power().await;
//     let p3_ts = now_timestamp().millis();

//     let power_4 = ina260_4.power().await;
//     let p4_ts = now_timestamp().millis();

//     let mut voltage_slice = [0.0_f32; 4];
//     let v_ts_slice = [v1_ts, v2_ts, v3_ts, v4_ts];

//     let mut current_slice = [0.0_f32; 4];
//     let i_ts_slice = [i1_ts, i2_ts, i3_ts, i4_ts];

//     let mut power_slice = [0.0_f32; 4];
//     let p_ts_slice = [p1_ts, p2_ts, p3_ts, p4_ts];

//     match voltage_1 {
//         Ok(voltage) => {
//             voltage_slice[0] = voltage;
//         }
//         Err(i2c_error) => {
//             error!("V1 Err: {}", i2c_error);
//             voltage_slice[0] = f32::NAN;
//         }
//     }
//     match voltage_2 {
//         Ok(voltage) => {
//             voltage_slice[1] = voltage;
//         }
//         Err(i2c_error) => {
//             error!("V2 Err: {}", i2c_error);
//             voltage_slice[1] = f32::NAN;
//         }
//     }
//     match voltage_3 {
//         Ok(voltage) => {
//             voltage_slice[2] = voltage;
//         }
//         Err(i2c_error) => {
//             error!("V3 Err: {}", i2c_error);
//             voltage_slice[2] = f32::NAN;
//         }
//     }
//     match voltage_4 {
//         Ok(voltage) => {
//             voltage_slice[3] = voltage;
//         }
//         Err(i2c_error) => {
//             error!("V4 Err: {}", i2c_error);
//             voltage_slice[3] = f32::NAN;
//         }
//     }
//     match current_1 {
//         Ok(current) => {
//             current_slice[0] = current;
//         }
//         Err(i2c_error) => {
//             error!("I1 Err: {}", i2c_error);
//             current_slice[0] = f32::NAN;
//         }
//     }
//     match current_2 {
//         Ok(current) => {
//             current_slice[1] = current;
//         }
//         Err(i2c_error) => {
//             error!("I2 Err: {}", i2c_error);
//             current_slice[1] = f32::NAN;
//         }
//     }
//     match current_3 {
//         Ok(current) => {
//             current_slice[2] = current;
//         }
//         Err(i2c_error) => {
//             error!("I3 Err: {}", i2c_error);
//             current_slice[2] = f32::NAN;
//         }
//     }
//     match current_4 {
//         Ok(current) => {
//             current_slice[3] = current;
//         }
//         Err(i2c_error) => {
//             error!("I4 Err: {}", i2c_error);
//             current_slice[3] = f32::NAN;
//         }
//     }
//     match power_1 {
//         Ok(power) => {
//             power_slice[0] = power;
//         }
//         Err(i2c_error) => {
//             error!("P1 Err: {}", i2c_error);
//             current_slice[0] = f32::NAN;
//         }
//     }
//     match power_2 {
//         Ok(power) => {
//             power_slice[1] = power;
//         }
//         Err(i2c_error) => {
//             error!("P2 Err: {}", i2c_error);
//             power_slice[1] = f32::NAN;
//         }
//     }
//     match power_3 {
//         Ok(power) => {
//             power_slice[2] = power;
//         }
//         Err(i2c_error) => {
//             error!("P3 Err: {}", i2c_error);
//             power_slice[2] = f32::NAN;
//         }
//     }
//     match power_4 {
//         Ok(power) => {
//             power_slice[3] = power;
//         }
//         Err(i2c_error) => {
//             error!("P4 Err: {}", i2c_error);
//             power_slice[3] = f32::NAN;
//         }
//     }
//     (
//         (v_ts_slice, i_ts_slice, p_ts_slice),
//         (voltage_slice, current_slice, power_slice),
//     )
// }


pub async fn read_photodiode(mut ctx: read_photodiode::Context<'_>)
{
    let mut i: usize = 0;

    loop
    {
        i = i + 1;

        if (i % 4) == 0
        {
            match ctx.local.mp_channel
            {
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
