use bin_packets::devices::DeviceIdentifier;
use bin_packets::packets::status::Status;
use bin_packets::packets::ApplicationPacket;
use bincode::config::standard;
use bincode::encode_into_slice;

use bmi323::{AccelConfig, GyroConfig};
use bmm350::MagConfig;
use defmt::{error, info};
use embedded_hal::digital::{InputPin, StatefulOutputPin};

use crate::device_constants::AvionicsI2cBus;
use crate::phases::{Modes, RelayServoStatus};
use crate::{app::*, device_constants::MotorI2cBus, Mono};
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

use tinyframe::buffer::FrameIter;
pub async fn radio_send(mut ctx: radio_send::Context<'_>) {
    let radio = ctx.local.radio;
    let mut buf_len = 0;
    let mut outgoing_packet_bytes = [0u8; 512];

    loop {
        // info!("Radio Send Task Operating");
        // First, drain outgoing packets until we run out of space in the outgoing packet bytes
        ctx.shared.data.lock(|data| {
            while let Some(packet) = data.pop_front() {
                if let Ok(w) =
                    encode_into_slice(packet, &mut outgoing_packet_bytes[buf_len..], standard())
                {
                    buf_len += w;
                    if buf_len == outgoing_packet_bytes.len() {
                        break;
                    }
                } else {
                    // no room → push it back and break
                    data.push_front(packet).ok();
                    break;
                }
            }
        });

        // Iter over bytes
        let mut frame_bytes = [0u8; 200];
        for frame in FrameIter::first(&outgoing_packet_bytes[..buf_len]) {
            let written = frame.encode_into_slice(&mut frame_bytes).unwrap();
            let bytes = &frame_bytes[..written];
            info!("Bytes: {:?}", bytes);
            for chunk in bytes.chunks(16) {
                let write = radio.write_all(chunk);
                match write {
                    Ok(_) => {
                        info!("Wrote chunk: {:?}", chunk);
                    }
                    Err(e) => {
                        error!("Error writing chunk: {:?}", e);
                    }
                }

                Mono::delay(40.millis()).await;
            }
            // info!("Sent Frame");
        }
        // Clear outgoing packet bytes
        buf_len = 0;
        Mono::delay(25.millis()).await;
    }
}

use rp235x_pac::interrupt;
#[interrupt]
unsafe fn I2C0_IRQ() {
    MotorI2cBus::on_interrupt();
}

use crate::phases::mode::{FLUTTER_COUNT, FLUTTER_START_TIME, SERVO_DISABLE_DELAY};
pub async fn mode_sequencer(ctx: mode_sequencer::Context<'_>) {
    let mut mode_start = Mono::now();
    let mut relay_status = false;
    ctx.local.relay_servo.enable();
    ctx.local.flap_servo.enable();
    ctx.local.flap_servo.deg_0();
    ctx.local.relay_servo.deg_0();
    let mut relay_flutter_status = RelayServoStatus::Open;
    let mut flutter_count = 0;
    let mut end_task = false;

    // Wait for RBF removal
    while ctx.local.rbf.is_high().unwrap() {
        Mono::delay(100.millis()).await;
    }

    loop {
        if !end_task {
            if !relay_status {
                // flap_status = Modes::open_flaps_sequence(mode_start, ctx.local.flap_servo).await;
                relay_status =
                    Modes::relay_eject_servo_sequence(mode_start, ctx.local.relay_servo).await;
            } else {
                Mono::delay(FLUTTER_START_TIME.millis()).await;
                if flutter_count < FLUTTER_COUNT {
                    mode_start = Mono::now();
                    // flap_flutter_status = Modes::flap_flutter_sequence(
                    //     mode_start,
                    //     flap_flutter_status,
                    //     ctx.local.flap_servo,
                    // )
                    // .await;
                    relay_flutter_status = Modes::relay_flutter_sequence(
                        mode_start,
                        relay_flutter_status,
                        ctx.local.relay_servo,
                    )
                    .await;
                    flutter_count += 1;
                } else {
                    // ctx.local.flap_servo.deg_0();
                    ctx.local.relay_servo.deg_0();
                    Mono::delay(SERVO_DISABLE_DELAY.millis()).await;
                    // ctx.local.flap_servo.disable();
                    ctx.local.relay_servo.disable();
                    end_task = true;
                }
            }
            Mono::delay(5_u64.millis()).await;
        } else {
            info!("Mode Sequencer Complete");
            Mono::delay(100000_u64.millis()).await;
        }
    }
}

pub async fn ina_sample(mut ctx: ina_sample::Context<'_>, _i2c: &'static Arbiter<MotorI2cBus>) {
    info!("INA Sample Task Started");
    if let Err(e) = ctx.local.ina260_1.init().await {
        error!("Error initializing INA 1: {:?}", e);
    }
    Mono::delay(2_u64.millis()).await;

    if let Err(e) = ctx.local.ina260_2.init().await {
        error!("Error initializing INA 2: {:?}", e);
    }
    Mono::delay(2_u64.millis()).await;

    if let Err(e) = ctx.local.ina260_3.init().await {
        error!("Error initializing INA 3: {:?}", e);
    }
    Mono::delay(2_u64.millis()).await;
    if let Err(e) = ctx.local.ina260_4.init().await {
        error!("Error initializing INA 4: {:?}", e);
    }
    Mono::delay(2_u64.millis()).await;

    ctx.local
        .ina260_1
        .set_operating_mode(ina260_terminus::OperMode::SCBVC)
        .await
        .ok();
    ctx.local
        .ina260_2
        .set_operating_mode(ina260_terminus::OperMode::SCBVC)
        .await
        .ok();
    ctx.local
        .ina260_3
        .set_operating_mode(ina260_terminus::OperMode::SCBVC)
        .await
        .ok();
    ctx.local
        .ina260_4
        .set_operating_mode(ina260_terminus::OperMode::SCBVC)
        .await
        .ok();

    loop {
        let ina_samples = ina_data_handle(
            ctx.local.ina260_1,
            ctx.local.ina260_2,
            ctx.local.ina260_3,
            ctx.local.ina260_4,
        )
        .await;
        ctx.shared.data.lock(|data| {
            let voltages_packet = ApplicationPacket::VoltageData {
                timestamp: ina_samples.0 .0,
                voltage: ina_samples.1 .0,
            };
            let current_packet = ApplicationPacket::CurrentData {
                timestamp: ina_samples.0 .1,
                current: ina_samples.1 .1,
            };
            let power_packet = ApplicationPacket::PowerData {
                timestamp: ina_samples.0 .2,
                power: ina_samples.1 .2,
            };
            info!("Voltage Packet: {}", voltages_packet);
            info!("Current Packet: {}", current_packet);
            info!("Power Packet: {}", power_packet);
            data.push_back(voltages_packet).ok();
            data.push_back(current_packet).ok();
            data.push_back(power_packet).ok();
        });
        Mono::delay(250.millis()).await;
    }
}

pub async fn sample_sensors(
    mut ctx: sample_sensors::Context<'_>,
    _avionics_i2c: &'static Arbiter<AvionicsI2cBus>,
) {
    ctx.local.bme280.init().await.ok();
    ctx.local.bmi323.init().await.ok();
    let accel_config = AccelConfig::builder().mode(bmi323::AccelerometerPowerMode::Normal);
    ctx.local
        .bmi323
        .set_accel_config(accel_config.build())
        .await
        .ok();
    let gyro_config = GyroConfig::builder().mode(bmi323::GyroscopePowerMode::Normal);
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
        // info!("T, P, H: {}, {}, {}", env.1, env.2, env.3);

        // let (photoresistor_1, photoresistor_2, photoresistor_3, photoresistor_4, photoresistor_5, photoresistor_6, photoresistor_7, photoresistor_8) = photoresistors_handle(ctx.local.adc, ctx.local.adc_photoresistors, ctx.local.mux).await;
        // info!("Photoresistors:  {}, {}, {}, {}, {}, {}, {}, {}", photoresistor_1, photoresistor_2, photoresistor_3, photoresistor_4, photoresistor_5, photoresistor_6, photoresistor_7, photoresistor_8);
        // let photoresistor_packet = ApplicationPacket::PhotoresistorData {
        //     timestamp: now_timestamp().millis(),
        //     vector: [photoresistor_1,photoresistor_2,photoresistor_3,photoresistor_4,photoresistor_5,photoresistor_6,photoresistor_7,photoresistor_8]
        // };
        Mono::delay(250_u64.millis()).await;
    }
}

pub async fn inertial_nav(_ctx: inertial_nav::Context<'_>) {
    loop {
        // info!("Inertial Navigation");
        Mono::delay(250_u64.millis()).await;
    }
}

// use rp235x_hal::adc::{Adc, AdcPin};
// use cd74hc4067::{CD74HC4067, Channel::Channel5, Channel::Channel4, Channel::Channel3, Channel::Channel2, Channel::Channel6, Channel::Channel7, Channel::Channel8, Channel::Channel9};
// use crate::{MuxS0Pin, MuxS1Pin, MuxS2Pin, MuxS3Pin, MuxEPin};
// use embedded_hal_0_2::adc::OneShot;
// async fn photoresistors_handle(mut adc: &mut Adc, mut adc_pin: rp235x_hal::adc::AdcPin<Pin<rp235x_hal::gpio::bank0::Gpio40, rp235x_hal::gpio::FunctionSioInput, rp235x_hal::gpio::PullNone>>, mut mux: CD74HC4067<Pin<MuxS0Pin, rp235x_hal::gpio::FunctionSio<rp235x_hal::gpio::SioOutput>, rp235x_hal::gpio::PullDown>, Pin<MuxS1Pin, rp235x_hal::gpio::FunctionSio<rp235x_hal::gpio::SioOutput>, rp235x_hal::gpio::PullDown>, Pin<MuxS2Pin, rp235x_hal::gpio::FunctionSio<rp235x_hal::gpio::SioOutput>, rp235x_hal::gpio::PullDown>, Pin<MuxS3Pin, rp235x_hal::gpio::FunctionSio<rp235x_hal::gpio::SioOutput>, rp235x_hal::gpio::PullDown>, Pin<MuxEPin, rp235x_hal::gpio::FunctionSio<rp235x_hal::gpio::SioOutput>, rp235x_hal::gpio::PullDown>>)->(u16, u16, u16, u16, u16, u16, u16, u16) {
//     // This function should handle the photoresistors and return their values
//     let mut photoresistor_values: [u16; 8] = [0; 8];
//     let mut temperature_sensor = adc.take_temp_sensor().unwrap();
//     let channels = [Channel5, Channel4, Channel3, Channel2, Channel6, Channel7, Channel8, Channel9];
//     for i in 0..8 {
//         let pin = &channels[i];
//         mux.set_pin(pin);
//         Mono::delay(10_u64.millis()).await; // Allow mux to settle
//         let adc_temp = adc.take_temp_sensor().unwrap();
//         let value = adc.read(&mut adc_pin).unwrap_or(0);
//         photoresistor_values[i] = value;
//     }
//     return (photoresistor_values[0], photoresistor_values[1], photoresistor_values[2], photoresistor_values[3], photoresistor_values[4], photoresistor_values[5], photoresistor_values[6], photoresistor_values[7]);
// }

// Sample Functions
use crate::peripherals::async_i2c::AsyncI2c;
use ina260_terminus::AsyncINA260;
use rp235x_hal::{
    gpio::{Pin, PullUp},
    I2C,
};
use rtic_sync::arbiter::i2c::ArbiterDevice;
#[allow(clippy::type_complexity)]
async fn ina_data_handle(
    ina260_1: &mut AsyncINA260<
        ArbiterDevice<
            '_,
            AsyncI2c<
                I2C<
                    rp235x_pac::I2C0,
                    (
                        Pin<rp235x_hal::gpio::bank0::Gpio16, rp235x_hal::gpio::FunctionI2c, PullUp>,
                        Pin<rp235x_hal::gpio::bank0::Gpio17, rp235x_hal::gpio::FunctionI2c, PullUp>,
                    ),
                >,
            >,
        >,
        Mono,
    >,
    ina260_2: &mut AsyncINA260<
        ArbiterDevice<
            '_,
            AsyncI2c<
                I2C<
                    rp235x_pac::I2C0,
                    (
                        Pin<rp235x_hal::gpio::bank0::Gpio16, rp235x_hal::gpio::FunctionI2c, PullUp>,
                        Pin<rp235x_hal::gpio::bank0::Gpio17, rp235x_hal::gpio::FunctionI2c, PullUp>,
                    ),
                >,
            >,
        >,
        Mono,
    >,
    ina260_3: &mut AsyncINA260<
        ArbiterDevice<
            '_,
            AsyncI2c<
                I2C<
                    rp235x_pac::I2C0,
                    (
                        Pin<rp235x_hal::gpio::bank0::Gpio16, rp235x_hal::gpio::FunctionI2c, PullUp>,
                        Pin<rp235x_hal::gpio::bank0::Gpio17, rp235x_hal::gpio::FunctionI2c, PullUp>,
                    ),
                >,
            >,
        >,
        Mono,
    >,
    ina260_4: &mut AsyncINA260<
        ArbiterDevice<
            '_,
            AsyncI2c<
                I2C<
                    rp235x_pac::I2C0,
                    (
                        Pin<rp235x_hal::gpio::bank0::Gpio16, rp235x_hal::gpio::FunctionI2c, PullUp>,
                        Pin<rp235x_hal::gpio::bank0::Gpio17, rp235x_hal::gpio::FunctionI2c, PullUp>,
                    ),
                >,
            >,
        >,
        Mono,
    >,
) -> (
    ([u64; 4], [u64; 4], [u64; 4]),
    ([f32; 4], [f32; 4], [f32; 4]),
) {
    let voltage_1 = ina260_1.voltage().await;
    let v1_ts = now_timestamp().millis();

    let voltage_2 = ina260_2.voltage().await;
    let v2_ts = now_timestamp().millis();

    let voltage_3 = ina260_3.voltage().await;
    let v3_ts = now_timestamp().millis();

    let voltage_4 = ina260_4.voltage().await;
    let v4_ts = now_timestamp().millis();

    let current_1 = ina260_1.current().await;
    let i1_ts = now_timestamp().millis();

    let current_2 = ina260_2.current().await;
    let i2_ts = now_timestamp().millis();

    let current_3 = ina260_3.current().await;
    let i3_ts = now_timestamp().millis();

    let current_4 = ina260_4.current().await;
    let i4_ts = now_timestamp().millis();

    let power_1 = ina260_1.power().await;
    let p1_ts = now_timestamp().millis();

    let power_2 = ina260_2.power().await;
    let p2_ts = now_timestamp().millis();

    let power_3 = ina260_3.power().await;
    let p3_ts = now_timestamp().millis();

    let power_4 = ina260_4.power().await;
    let p4_ts = now_timestamp().millis();

    let mut voltage_slice = [0.0_f32; 4];
    let v_ts_slice = [v1_ts, v2_ts, v3_ts, v4_ts];

    let mut current_slice = [0.0_f32; 4];
    let i_ts_slice = [i1_ts, i2_ts, i3_ts, i4_ts];

    let mut power_slice = [0.0_f32; 4];
    let p_ts_slice = [p1_ts, p2_ts, p3_ts, p4_ts];

    match voltage_1 {
        Ok(voltage) => {
            voltage_slice[0] = voltage;
        }
        Err(i2c_error) => {
            error!("V1 Err: {}", i2c_error);
            voltage_slice[0] = f32::NAN;
        }
    }
    match voltage_2 {
        Ok(voltage) => {
            voltage_slice[1] = voltage;
        }
        Err(i2c_error) => {
            error!("V2 Err: {}", i2c_error);
            voltage_slice[1] = f32::NAN;
        }
    }
    match voltage_3 {
        Ok(voltage) => {
            voltage_slice[2] = voltage;
        }
        Err(i2c_error) => {
            error!("V3 Err: {}", i2c_error);
            voltage_slice[2] = f32::NAN;
        }
    }
    match voltage_4 {
        Ok(voltage) => {
            voltage_slice[3] = voltage;
        }
        Err(i2c_error) => {
            error!("V4 Err: {}", i2c_error);
            voltage_slice[3] = f32::NAN;
        }
    }
    match current_1 {
        Ok(current) => {
            current_slice[0] = current;
        }
        Err(i2c_error) => {
            error!("I1 Err: {}", i2c_error);
            current_slice[0] = f32::NAN;
        }
    }
    match current_2 {
        Ok(current) => {
            current_slice[1] = current;
        }
        Err(i2c_error) => {
            error!("I2 Err: {}", i2c_error);
            current_slice[1] = f32::NAN;
        }
    }
    match current_3 {
        Ok(current) => {
            current_slice[2] = current;
        }
        Err(i2c_error) => {
            error!("I3 Err: {}", i2c_error);
            current_slice[2] = f32::NAN;
        }
    }
    match current_4 {
        Ok(current) => {
            current_slice[3] = current;
        }
        Err(i2c_error) => {
            error!("I4 Err: {}", i2c_error);
            current_slice[3] = f32::NAN;
        }
    }
    match power_1 {
        Ok(power) => {
            power_slice[0] = power;
        }
        Err(i2c_error) => {
            error!("P1 Err: {}", i2c_error);
            current_slice[0] = f32::NAN;
        }
    }
    match power_2 {
        Ok(power) => {
            power_slice[1] = power;
        }
        Err(i2c_error) => {
            error!("P2 Err: {}", i2c_error);
            power_slice[1] = f32::NAN;
        }
    }
    match power_3 {
        Ok(power) => {
            power_slice[2] = power;
        }
        Err(i2c_error) => {
            error!("P3 Err: {}", i2c_error);
            power_slice[2] = f32::NAN;
        }
    }
    match power_4 {
        Ok(power) => {
            power_slice[3] = power;
        }
        Err(i2c_error) => {
            error!("P4 Err: {}", i2c_error);
            power_slice[3] = f32::NAN;
        }
    }
    (
        (v_ts_slice, i_ts_slice, p_ts_slice),
        (voltage_slice, current_slice, power_slice),
    )
}
