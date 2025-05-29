use bin_packets::device::PacketWriter;
use bin_packets::devices::DeviceIdentifier;
use bin_packets::packets::status::Status;
use bin_packets::packets::ApplicationPacket;
use bme280_rs::{Configuration, Oversampling, SensorMode};
use defmt::{info, warn, error};
use embedded_hal::digital::StatefulOutputPin;
use fugit::{ExtU64, Instant};
use rtic::Mutex;
use rtic_monotonics::Monotonic;
use rtic_sync::arbiter::Arbiter;

use crate::device_constants::servos::{FlapServo, FLAP_SERVO_LOCKED, FLAP_SERVO_UNLOCKED};
use crate::device_constants::AvionicsI2cBus;
use crate::phases::{FlapServoStatus, Modes, RelayServoStatus, StateMachineListener};
use crate::{app::*, device_constants::MotorI2cBus, Mono};

pub async fn heartbeat(mut ctx: heartbeat::Context<'_>) {
    let mut sequence_number = 0;
    loop {
        _ = ctx.local.led.toggle();

        let status = Status::new(DeviceIdentifier::Icarus, now_timestamp(), sequence_number);

        let packet_send = ctx.shared.radio.lock(|radio| radio.write(status).err());

        // if let Some(err) = packet_send {
        //     warn!("Failed to send heartbeat: {}", err);
        // }

        sequence_number = sequence_number.wrapping_add(1);

        Mono::delay(300_u64.millis()).await;
    }
}

pub async fn radio_send(mut ctx: radio_send::Context<'_>) {
    loop {
        ctx.shared.data.lock(|ina_data| {
            ctx.shared.radio.lock(|radio| {
                // GET PACKETS FROM INA DATA AND SEND
                let packet_1 = ina_data.i1_buffer.first();
                // let packet_2 = ina_data.i2_buffer.first();
                // let packet_3 = ina_data.i3_buffer.first();
                match packet_1 {
                    Some(packet_info) => {
                        info!("Data Write: {:?}", packet_info);
                        let radio_write_result = radio.write(*packet_info);
                        match radio_write_result {
                            Ok(radio_result) => {
                                // !TODO
                            }
                            Err(_) => {
                                // !TODO
                            }
                        }
                    }
                    None => {
                        // info!("No Packet To Send")
                    }
                }
            });
        });

        Mono::delay(100_u64.millis()).await;
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
use rp235x_hal::clocks;

use crate::phases::mode::{FLUTTER_COUNT, FLUTTER_START_TIME, SERVO_DISABLE_DELAY};
pub async fn mode_sequencer(mut ctx: mode_sequencer::Context<'_>) {
    let mut status = 0;
    let mut iteration = 0;
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
            relay_status = Modes::relay_eject_servo_sequence(mode_start, ctx.local.relay_servo).await;
        } 
        else {
            Mono::delay(FLUTTER_START_TIME.millis()).await;
            if flutter_count < FLUTTER_COUNT{
                mode_start = Mono::now();
                flap_flutter_status = Modes::flap_flutter_sequence(mode_start, flap_flutter_status, ctx.local.flap_servo).await;
                relay_flutter_status = Modes::relay_flutter_sequence(mode_start, relay_flutter_status, ctx.local.relay_servo).await;
                flutter_count += 1;
            }
            else{
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
        match voltage1_result{
            Ok(voltage_1)=>{
                info!("1: {:?}", voltage_1);
                let vs1 = ApplicationPacket::VoltageData {
                    name:1,
                    time_stamp: ts,
                    voltage: Some(voltage_1),
                };
                ctx.shared.data.lock(|ina_data| {
                        ina_data.v1_buffer.write(vs1);
                    }
                );
            }
            Err(_)=>{
                info!("Wack 1");
            }
        }
        let voltage2_result = ctx.local.ina260_2.voltage_split().await;
            match voltage2_result{
                Ok(voltage_2)=>{
                    info!("2: {:?}", voltage_2);
                    let vs2 = ApplicationPacket::VoltageData {
                        name:2,
                        time_stamp: ts,
                        voltage: Some(voltage_2),
                    };
                    ctx.shared.data.lock(|ina_data| {
                            ina_data.v2_buffer.write(vs2);
                        }
                    );
                }
                Err(_)=>{
                    info!("Wack 2");
                }
            }
        let voltage3_result = ctx.local.ina260_3.voltage_split().await;
        match voltage3_result{
            Ok(voltage_3)=>{
                info!("3: {:?}", voltage_3);
                let vs3 = ApplicationPacket::VoltageData {
                    name: 3,
                    time_stamp: ts,
                    voltage: Some(voltage_3),
                };
                ctx.shared.data.lock(|ina_data| {
                        ina_data.v3_buffer.write(vs3);
                    }
                );
            }
            Err(_)=>{
                info!("Wack 3");
            }
        }
        Mono::delay(100_u64.millis()).await;
    }
}


use uom::si::pressure::pascal;
use uom::si::ratio::percent;
use uom::si::thermodynamic_temperature::degree_celsius;

pub async fn sample_sensors(
    ctx: sample_sensors::Context<'_>,
    _avionics_i2c: &'static Arbiter<AvionicsI2cBus>,
) {
    let bme_on = ctx.local.bme280.init().await;
    match bme_on{
        Ok(_)=>{
            
        }
        Err(i2c_error)=>{
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
    match bme_id{
        Ok(id)=>{
            info!("BME280 ID: {}", id);
        }
        Err(i2c_error)=>{
            info!("I2CError: {}", i2c_error)
        }
    }

    Mono::delay(10_u64.millis()).await; // !TODO (Remove me if no effect) Delaying preemptive to other processes just in case...
    let bmi323_init_result = ctx.local.bmi323.init().await;
    match bmi323_init_result{
        Ok(_)=>{
            info!("BMI Initialized");
        }
        Err(_)=>{
            error!("BMI Unininitialized");
        }
    }


    Mono::delay(10_u64.millis()).await;


    loop {
        let bmm350_init_result = ctx.local.bmm350.init().await;
        match bmm350_init_result{
            Ok(_)=>{
                info!("BMM Initialized");
            }
            Err(_)=>{
                error!("BMM Unininitialized");
            }
        }

        let sample_result = ctx.local.bme280.read_sample().await;
        match sample_result{
            Ok(sample)=>{
                let temperature = sample.temperature.unwrap();
                let humidity = sample.humidity.unwrap();
                let pressure = sample.pressure.unwrap();
                info!("Sample: ┳ Temperature: {} C", temperature);
                info!("        ┣ Humidity: {} %", humidity);
                info!("        ┗ Pressure: {} hPa", pressure);
            }
            Err(i2c_error)=>{
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

