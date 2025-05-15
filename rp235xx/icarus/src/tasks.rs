use bin_packets::device::PacketIO;
use bin_packets::devices::DeviceIdentifier;
use bin_packets::packets::status::Status;
use bin_packets::packets::ApplicationPacket;
use bme280_rs::{Configuration, Oversampling, SensorMode};
use defmt::{info, warn};
use embedded_hal::digital::StatefulOutputPin;
use fugit::ExtU64;
use rtic::Mutex;
use rtic_monotonics::Monotonic;
use rtic_sync::arbiter::Arbiter;

use crate::device_constants::AvionicsI2cBus;
use crate::phases::{StateMachineListener, Modes};
use crate::{app::*, device_constants::MotorI2cBus, Mono};

pub async fn heartbeat(mut ctx: heartbeat::Context<'_>) {
    let mut sequence_number = 0;
    loop {
        _ = ctx.local.led.toggle();

        let status = Status::new(DeviceIdentifier::Icarus, now_timestamp(), sequence_number);

        let packet_send = ctx
            .shared
            .radio
            .lock(|radio| radio.write_into(status).err());

        if let Some(err) = packet_send {
            warn!("Failed to send heartbeat: {:?}", err);
        }

        sequence_number = sequence_number.wrapping_add(1);

        Mono::delay(300_u64.millis()).await;
    }
}

pub fn uart_interrupt(mut ctx: uart_interrupt::Context<'_>) {
    ctx.shared.radio.lock(|radio| {
        radio.update().ok();
    });
}

pub async fn radio_send(mut ctx: radio_send::Context<'_>) {
    loop {
        ctx.shared.ina_data.lock(|ina_data| {
            ctx.shared.radio.lock(|radio| {
                // GET PACKETS FROM INA DATA AND SEND
                let packet = ina_data.i1_buffer.first();
                match packet {
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
                        info!("No Packet To Send")
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
pub async fn mode_sequencer(mut ctx: mode_sequencer::Context<'_>){
    let mut status = 0;
    let mut iteration = 0;
    let mut mode_start = Mono::now();    
    
    let mut flap_status = false;
    let mut relay_status = false;
    ctx.local.relay_servo.enable();
    ctx.local.flap_servo.enable();
    ctx.local.flap_servo.deg_0();
    ctx.local.relay_servo.deg_0();
    loop{
        if flap_status == false{
            flap_status = Modes::open_flaps_sequence(mode_start, ctx.local.flap_servo).await;
        }
        
        if relay_status == false{
            relay_status = Modes::eject_servo_sequence(mode_start, ctx.local.relay_servo).await;
        }
        
        Mono::delay(100_u64.millis()).await;
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
        let voltage_1 = ctx.local.ina260_1.voltage_split().await.ok();
        let current_1 = ctx.local.ina260_1.current_split().await.ok();
        let power_1 = ctx.local.ina260_1.power_split().await.ok();
        let voltage_2 = ctx.local.ina260_2.voltage_split().await.ok();
        let current_2 = ctx.local.ina260_2.current_split().await.ok();
        let power_2 = ctx.local.ina260_2.power_split().await.ok();
        let voltage_3 = ctx.local.ina260_3.voltage_split().await.ok();
        let current_3 = ctx.local.ina260_3.current_split().await.ok();
        let power_3 = ctx.local.ina260_3.power_split().await.ok();

        let vs1 = ApplicationPacket::VoltageData {
            time_stamp: ts,
            power: voltage_1,
        };
        let vs2 = ApplicationPacket::VoltageData {
            time_stamp: ts,
            power: voltage_2,
        };
        let vs3 = ApplicationPacket::VoltageData {
            time_stamp: ts,
            power: voltage_3,
        };
        let cur1 = ApplicationPacket::CurrentData {
            time_stamp: ts,
            power: current_1,
        };
        let cur2 = ApplicationPacket::CurrentData {
            time_stamp: ts,
            power: current_2,
        };
        let cur3 = ApplicationPacket::CurrentData {
            time_stamp: ts,
            power: current_3,
        };
        let pow1 = ApplicationPacket::PowerData {
            time_stamp: ts,
            power: power_1,
        };
        let pow2 = ApplicationPacket::PowerData {
            time_stamp: ts,
            power: power_2,
        };
        let pow3 = ApplicationPacket::PowerData {
            time_stamp: ts,
            power: power_3,
        };

        ctx.shared.ina_data.lock(|ina_data| {
            ina_data.v1_buffer.write(vs1);
            ina_data.v2_buffer.write(vs2);
            ina_data.v3_buffer.write(vs3);
            ina_data.i1_buffer.write(cur1);
            ina_data.i2_buffer.write(cur2);
            ina_data.i3_buffer.write(cur3);
            ina_data.p1_buffer.write(pow1);
            ina_data.p2_buffer.write(pow2);
            ina_data.p3_buffer.write(pow3);
        });
        Mono::delay(100_u64.millis()).await;
    }
}

pub async fn sample_sensors(
    ctx: sample_sensors::Context<'_>,
    _avionics_i2c: &'static Arbiter<AvionicsI2cBus>,
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

    loop {
        // if let Ok(Some(temperature)) = ctx.local.bme280.read_temperature().await {
        //     info!("Temperature: {}", temperature);
        // }
        // if let Ok(Some(pressure)) = ctx.local.bme280.read_pressure().await {
        //     info!("Pressure: {}", pressure);
        // }
        // if let Ok(Some(humidity)) = ctx.local.bme280.read_humidity().await {
        //     info!("Humidity: {}", humidity);
        // }
        Mono::delay(250_u64.millis()).await;
    }
}

pub async fn inertial_nav(_ctx: inertial_nav::Context<'_>) {
    loop {
        // info!("Inertial Navigation");
        Mono::delay(250_u64.millis()).await;
    }
}