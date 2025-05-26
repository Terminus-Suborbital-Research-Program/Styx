use bin_packets::device::PacketWriter;
use bin_packets::devices::DeviceIdentifier;
use bin_packets::packets::status::Status;
use bin_packets::packets::ApplicationPacket;
use bme280_rs::{Configuration, Oversampling, SensorMode};
use defmt::{error, info};
use embedded_hal::digital::StatefulOutputPin;
use fugit::ExtU64;
use rtic::Mutex;
use rtic_monotonics::Monotonic;
use rtic_sync::arbiter::Arbiter;

use crate::device_constants::AvionicsI2cBus;
use crate::phases::{FlapServoStatus, Modes, RelayServoStatus, StateMachineListener};
use crate::{app::*, device_constants::MotorI2cBus, Mono};

pub async fn heartbeat(mut ctx: heartbeat::Context<'_>) {
    let mut sequence_number = 0;
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

pub async fn radio_send(mut ctx: radio_send::Context<'_>) {
    let radio = ctx.local.radio;
    loop {
        while let Some(packet) = ctx.shared.data.lock(|buffer| buffer.pop_front()) {
            if let Err(e) = radio.write(packet) {
                error!("Could not send packet: {:?}", defmt::Debug2Format(&e));
            }
            // TODO: Test this duration stability
            Mono::delay(50.millis()).await;
        }
    }
}

use rp235x_pac::interrupt;
#[interrupt]
unsafe fn I2C0_IRQ() {
    MotorI2cBus::on_interrupt();
}

// async fn flap_servo_close(mut servo: &mut IcarusServos){
//     servo.set_angle(0);
// }

use crate::phases::mode::{FLUTTER_COUNT, FLUTTER_START_TIME, SERVO_DISABLE_DELAY};
pub async fn mode_sequencer(ctx: mode_sequencer::Context<'_>) {
    let status = 0;
    let iteration = 0;
    let mut mode_start = Mono::now();

    let mut flap_status = false;
    let mut relay_status = false;
    ctx.local.relay_servo.enable();
    ctx.local.flap_servo.enable();
    ctx.local.flap_servo.deg_0();
    ctx.local.relay_servo.deg_0();
    let mut relay_flutter_status = RelayServoStatus::Open;
    let mut flap_flutter_status = FlapServoStatus::Open;
    let mut flutter_count = 0;
    loop {
        if flap_status == false {
            flap_status = Modes::open_flaps_sequence(mode_start, ctx.local.flap_servo).await;
            relay_status =
                Modes::relay_eject_servo_sequence(mode_start, ctx.local.relay_servo).await;
        } else {
            Mono::delay(FLUTTER_START_TIME.millis()).await;
            if flutter_count < FLUTTER_COUNT {
                mode_start = Mono::now();
                flap_flutter_status = Modes::flap_flutter_sequence(
                    mode_start,
                    flap_flutter_status,
                    ctx.local.flap_servo,
                )
                .await;
                relay_flutter_status = Modes::relay_flutter_sequence(
                    mode_start,
                    relay_flutter_status,
                    ctx.local.relay_servo,
                )
                .await;
                flutter_count += 1;
            } else {
                ctx.local.flap_servo.deg_0();
                ctx.local.relay_servo.deg_0();
                Mono::delay(SERVO_DISABLE_DELAY.millis()).await;
                ctx.local.flap_servo.disable();
                ctx.local.relay_servo.disable();
            }
        }
        Mono::delay(5_u64.millis()).await;
    }
}

pub async fn motor_drivers(
    mut ctx: motor_drivers::Context<'_>,
    _i2c: &'static Arbiter<MotorI2cBus>,
    esc_state_listener: StateMachineListener,
) {
    info!("Motor Driver Task Started");

    ctx.local.ina260_1.init().await.ok();
    ctx.local.ina260_2.init().await.ok();
    ctx.local.ina260_3.init().await.ok();

    loop {
        let ts = Mono::now().ticks();

        let voltage1_result = ctx.local.ina260_1.voltage_split().await;
        match voltage1_result {
            Ok(voltage_1) => {
                info!("2: {:?}", voltage_1);
                let vs1 = ApplicationPacket::VoltageData {
                    name: 1,
                    time_stamp: ts,
                    voltage: Some(voltage_1),
                };
                ctx.shared.data.lock(|ina_data| {
                    ina_data.push_back(vs1).ok();
                });
            }
            Err(_) => {
                info!("Wack 1");
            }
        }
        let current_1 = ctx.local.ina260_1.current_split().await.ok();
        let power_1 = ctx.local.ina260_1.power_raw().await.ok();
        let voltage2_result = ctx.local.ina260_2.voltage_split().await;
        match voltage2_result {
            Ok(voltage_2) => {
                info!("2: {:?}", voltage_2);
                let vs2 = ApplicationPacket::VoltageData {
                    name: 2,
                    time_stamp: ts,
                    voltage: Some(voltage_2),
                };
                ctx.shared.data.lock(|ina_data| {
                    ina_data.push_back(vs2).ok();
                });
            }
            Err(_) => {
                info!("Wack 2");
            }
        }
        let current_2 = ctx.local.ina260_2.current_split().await.ok();
        let power_2 = ctx.local.ina260_2.power_split().await.ok();
        let voltage3_result = ctx.local.ina260_3.voltage_split().await;
        match voltage3_result {
            Ok(voltage_3) => {
                info!("3: {:?}", voltage_3);
                let vs3 = ApplicationPacket::VoltageData {
                    name: 3,
                    time_stamp: ts,
                    voltage: Some(voltage_3),
                };
                ctx.shared.data.lock(|ina_data| {
                    ina_data.push_back(vs3).ok();
                });
            }
            Err(_) => {
                info!("Wack 3");
            }
        }
        let current_3 = ctx.local.ina260_3.current_split().await.ok();
        let power_3 = ctx.local.ina260_3.power_split().await.ok();

        let cur1 = ApplicationPacket::CurrentData {
            name: 1,
            time_stamp: ts,
            current: current_1,
        };
        // let cur2 = ApplicationPacket::CurrentData {
        //     name: 2,
        //     time_stamp: ts,
        //     current: current_2,
        // };
        // let cur3 = ApplicationPacket::CurrentData {
        //     name: 3,
        //     time_stamp: ts,
        //     current: current_3,
        // };
        let pow1 = ApplicationPacket::PowerData {
            name: 1,
            time_stamp: ts,
            power: power_1,
        };
        // let pow2 = ApplicationPacket::PowerData {
        //     name: 2,
        //     time_stamp: ts,
        //     power: power_2,
        // };
        // let pow3 = ApplicationPacket::PowerData {
        //     name: 3,
        //     time_stamp: ts,
        //     power: power_3,
        // };

        // ina_data.i1_buffer.write(cur1);
        // // ina_data.i2_buffer.write(cur2);
        // // ina_data.i3_buffer.write(cur3);
        // ina_data.p1_buffer.write(pow1);
        // // ina_data.p2_buffer.write(pow2);
        // // ina_data.p3_buffer.write(pow3);
        Mono::delay(100_u64.millis()).await;
    }
}

pub async fn sample_sensors(
    ctx: sample_sensors::Context<'_>,
    _avionics_i2c: &'static Arbiter<AvionicsI2cBus>,
) {
    let bme_on = ctx.local.bme280.init().await;
    match bme_on {
        Ok(_) => {}
        Err(i2c_error) => {
            info!("BME Error: {}", i2c_error);
        }
    }
    Mono::delay(10_u64.millis()).await;
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

    let bme_id = ctx.local.bme280.chip_id().await;
    match bme_id {
        Ok(id) => {
            info!("BME280 ID: {}", id);
        }
        Err(i2c_error) => {
            info!("I2CError: {}", i2c_error)
        }
    }

    Mono::delay(10_u64.millis()).await; // !TODO (Remove me if no effect) Delaying preemptive to other processes just in case...
    let bmi323_init_result = ctx.local.bmi323.init().await;
    match bmi323_init_result {
        Ok(_) => {
            info!("BMI Initialized");
        }
        Err(_) => {
            error!("BMI Unininitialized");
        }
    }

    loop {
        let sample_result = ctx.local.bme280.read_sample().await;
        match sample_result {
            Ok(sample) => {
                let temperature = sample.temperature.unwrap();
                let humidity = sample.humidity.unwrap();
                let pressure = sample.pressure.unwrap();
                info!("Sample: ┳ Temperature: {} C", temperature);
                info!("        ┣ Humidity: {} %", humidity);
                info!("        ┗ Pressure: {} hPa", pressure);
            }
            Err(i2c_error) => {
                error!("I2C Error: {}", i2c_error)
            }
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
