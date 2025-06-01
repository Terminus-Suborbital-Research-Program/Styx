use bin_packets::device::PacketWriter;
use bin_packets::devices::DeviceIdentifier;
use bin_packets::packets::status::Status;
use bin_packets::packets::ApplicationPacket;
use bme280::{Configuration, Oversampling, SensorMode};
use defmt::{error, info};
use embedded_hal::digital::StatefulOutputPin;
use fugit::ExtU64;
use futures::join;
use heapless::Vec;
use rtic::Mutex;
use rtic_monotonics::Monotonic;
use rtic_sync::arbiter::Arbiter;

use crate::device_constants::AvionicsI2cBus;
use crate::phases::{FlapServoStatus, Modes, RelayServoStatus};
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
        if radio.waiting() > 0 {
            while ctx.shared.data.lock(|data| data.len()) < 1 {
                Mono::delay(100.millis()).await;
            }
        }

        ctx.shared.data.lock(|data| {
            while let Some(packet) = data.front() {
                if radio.add(packet).is_ok() {
                    let _ = data.pop_front();
                }
            }
        });

        if radio.waiting() > 0 {
            let written = radio.write(32).unwrap_or(0) as u64;
            Mono::delay((written / (9600 / 8)).secs()).await;
            info!("Wrote {} bytes to the buffer", written);
        }
    }
}

use rp235x_pac::interrupt;
#[interrupt]
unsafe fn I2C0_IRQ() {
    MotorI2cBus::on_interrupt();
}

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

pub async fn ina_sample(mut ctx: ina_sample::Context<'_>, _i2c: &'static Arbiter<MotorI2cBus>) {
    info!("Motor Driver Task Started");

    if let Err(e) = ctx.local.ina260_1.init().await {
        error!("Error initializing INA 1: {:?}", e);
    }
    Mono::delay(2_u64.millis()).await;

    if let Err(e) = ctx.local.ina260_2.init().await {
        error!("Error initializing INA 2: {:?}", e);
    }
    Mono::delay(2_u64.millis()).await;

    if let Err(e) = ctx.local.ina260_3.init().await {
        error!("Error initializing INA 1: {:?}", e);
    }
    Mono::delay(2_u64.millis()).await;

    ctx.local.ina260_1.set_operating_mode(ina260_terminus::OperMode::SCBVC).await;
    ctx.local.ina260_2.set_operating_mode(ina260_terminus::OperMode::SCBVC).await;
    ctx.local.ina260_3.set_operating_mode(ina260_terminus::OperMode::SCBVC).await;
    ctx.local.ina260_4.set_operating_mode(ina260_terminus::OperMode::SCBVC).await;

    loop {
        let voltages = join!(
            ctx.local.ina260_1.voltage(),
            ctx.local.ina260_2.voltage(),
            ctx.local.ina260_3.voltage(),
            ctx.local.ina260_4.voltage()
        );

        let currents = join!(
            ctx.local.ina260_1.current(),
            ctx.local.ina260_2.current(),
            ctx.local.ina260_3.current(),
            ctx.local.ina260_4.current()
        );

        let powers = join!(
            ctx.local.ina260_1.power(),
            ctx.local.ina260_2.power(),
            ctx.local.ina260_3.power(),
            ctx.local.ina260_4.power()
        );

        info!("Voltages: {}, {}, {}, {}", voltages.0, voltages.1, voltages.2, voltages.3);
        info!("Currents: {}, {}, {}, {}", currents.0, currents.1, currents.2, currents.3);
        info!("Powers: {}, {}, {}, {}", powers.0, powers.1, powers.2, powers.3);

        // let voltages = [
        //     items.0.unwrap_or(f32::NAN),
        //     items.1.unwrap_or(f32::NAN),
        //     items.2.unwrap_or(f32::NAN),
        // ];
        // let packet = ApplicationPacket::VoltageData {
        //     timestamp: epoch_ns(),
        //     voltage: voltages,
        // };

        // ctx.shared.data.lock(|vec| vec.push_back(packet).ok());

        Mono::delay(50.millis()).await;
    }
}

pub async fn sample_sensors(
    ctx: sample_sensors::Context<'_>,
    _avionics_i2c: &'static Arbiter<AvionicsI2cBus>,
) {
    // let bmi323_init_result = ctx.local.bmi323.init().await;
    // match bmi323_init_result {
    //     Ok(_) => {
    //         info!("BMI Initialized");
    //     }
    //     Err(_) => {
    //         error!("BMI Unininitialized");
    //     }
    // }

    // Mono::delay(10_u64.millis()).await; // !TODO (Remove me if no effect) Delaying preemptive to other processes just in case...
    // let bmi323_init_result = ctx.local.bmi323.init().await;
    // match bmi323_init_result{
    //     Ok(_)=>{
    //         info!("BMI Initialized");
    //     }
    //     Err(_)=>{
    //         error!("BMI Unininitialized");
    //     }
    // }
        // let bmi323_init_result = ctx.local.bmi323.init().await;
        // match bmi323_init_result{
        //     Ok(_)=>{
        //         info!("BMI Initialized");
        //     }
        //     Err(_)=>{
        //         error!("BMI Unininitialized");
        //     }
        // }
        let bme_on = ctx.local.bme280.init_with_config(&mut Mono, 
                Configuration::default()
                    .with_humidity_oversampling(Oversampling::Oversampling1X)
                    .with_pressure_oversampling(Oversampling::Oversampling16X)
                    .with_temperature_oversampling(Oversampling::Oversampling2X)
        ).await;
        match bme_on {
            Ok(_) => {}
            Err(i2c_error) => {
                error!("BME Error: {}", i2c_error);
            }
        }
        // ctx.local
        //     .bme280
        //     .set_sampling_configuration(
        //         Configuration::default()
        //             .with_temperature_oversampling(Oversampling::Oversample1)
        //             .with_pressure_oversampling(Oversampling::Oversample1)
        //             .with_humidity_oversampling(Oversampling::Oversample1)
        //             .with_sensor_mode(SensorMode::Normal),
        //     )
        //     .await
        //     .expect("Failed to configure BME280");
    // let bme_id = ctx.local.bme280.).await;
    // match bme_id {
    //     Ok(id) => {
    //         info!("BME280 ID: {}", id);
    //     }
    //     Err(i2c_error) => {
    //         error!("I2CError: {}", i2c_error)
    //     }
    // }
    // Mono::delay(10_u64.millis()).await;

    loop {
        let sample = ctx.local.bme280.measure(&mut Mono).await;
        match sample{
            Ok(values)=>{
                let temperature = values.temperature;
                let pressure = values.pressure;
                let humidity = values.humidity;
            }
            Err(error)=>{
                error!("BME280 Error: {}", error);
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
