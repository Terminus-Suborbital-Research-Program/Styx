use bin_packets::device::PacketWriter;
use bin_packets::devices::DeviceIdentifier;
use bin_packets::packets::status::Status;
use bin_packets::packets::ApplicationPacket;
use bme280_rs::{Configuration, Oversampling, SensorMode};
use defmt::{error, info};
use embedded_hal::digital::StatefulOutputPin;
use fugit::ExtU64;
use futures::join;
use heapless::Vec;
use rtic::Mutex;
use rtic_monotonics::Monotonic;
use rtic_sync::arbiter::Arbiter;
use uom::si::electric_potential::volt;
use uom::si::power;

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

    ctx.local.ina260_1.set_operating_mode(ina260_terminus::OperMode::SCBVC).await;
    ctx.local.ina260_2.set_operating_mode(ina260_terminus::OperMode::SCBVC).await;
    ctx.local.ina260_3.set_operating_mode(ina260_terminus::OperMode::SCBVC).await;
    ctx.local.ina260_4.set_operating_mode(ina260_terminus::OperMode::SCBVC).await;

    loop {
        let ina_samples = ina_data_handle(ctx.local.ina260_1, ctx.local.ina260_2, ctx.local.ina260_3, ctx.local.ina260_4).await;
        ctx.shared.data.lock(|data|{
            let voltages_packet = ApplicationPacket::VoltageData { timestamp: ina_samples.0.0, voltage: ina_samples.1.0};
            let current_packet = ApplicationPacket::CurrentData { timestamp: ina_samples.0.1, current: ina_samples.1.1};
            let power_packet = ApplicationPacket::PowerData { timestamp: ina_samples.0.2, power: ina_samples.1.2};
            data.push_back(voltages_packet);
            data.push_back(current_packet);
            data.push_back(power_packet);
        });
        Mono::delay(50.millis()).await;
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
            error!("BME Error: {}", i2c_error);
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

    // TODO: (Remove me if no effect) Delaying preemptive to other processes just in case...
    Mono::delay(10_u64.millis()).await;
    let bmi323_init_result = ctx.local.bmi323.init().await;
    match bmi323_init_result {
        Ok(_) => {
            info!("BMI Initialized");
        }
        Err(_) => {
            error!("BMI Unininitialized");
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
        let sample_result = ctx.local.bme280.read_sample().await;
        match sample_result {
            Ok(sample) => {
                // let temperature = sample.temperature.unwrap();
                // let humidity = sample.humidity.unwrap();
                // let pressure = sample.pressure.unwrap();
            //     info!("Sample: ┳ Temperature: {} C", temperature);
            //     info!("        ┣ Humidity: {} %", humidity);
            //     info!("        ┗ Pressure: {} hPa", pressure);
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

// Sample Functions
use ina260_terminus::AsyncINA260;
use rtic_sync::arbiter::i2c::ArbiterDevice;
use crate::peripherals::async_i2c::AsyncI2c;
use rp235x_hal::{I2C, gpio::{Pin, PullUp}};
async fn ina_data_handle(ina260_1: &mut AsyncINA260<ArbiterDevice<'_, AsyncI2c<I2C<rp235x_pac::I2C0, (Pin<rp235x_hal::gpio::bank0::Gpio16, rp235x_hal::gpio::FunctionI2c, PullUp>, Pin<rp235x_hal::gpio::bank0::Gpio17, rp235x_hal::gpio::FunctionI2c, PullUp>)>>>, Mono>, ina260_2: &mut AsyncINA260<ArbiterDevice<'_, AsyncI2c<I2C<rp235x_pac::I2C0, (Pin<rp235x_hal::gpio::bank0::Gpio16, rp235x_hal::gpio::FunctionI2c, PullUp>, Pin<rp235x_hal::gpio::bank0::Gpio17, rp235x_hal::gpio::FunctionI2c, PullUp>)>>>, Mono>, ina260_3: &mut AsyncINA260<ArbiterDevice<'_, AsyncI2c<I2C<rp235x_pac::I2C0, (Pin<rp235x_hal::gpio::bank0::Gpio16, rp235x_hal::gpio::FunctionI2c, PullUp>, Pin<rp235x_hal::gpio::bank0::Gpio17, rp235x_hal::gpio::FunctionI2c, PullUp>)>>>, Mono>, ina260_4: &mut AsyncINA260<ArbiterDevice<'_, AsyncI2c<I2C<rp235x_pac::I2C0, (Pin<rp235x_hal::gpio::bank0::Gpio16, rp235x_hal::gpio::FunctionI2c, PullUp>, Pin<rp235x_hal::gpio::bank0::Gpio17, rp235x_hal::gpio::FunctionI2c, PullUp>)>>>, Mono>) -> (([u64; 4], [u64; 4], [u64; 4]), ([f32; 4], [f32; 4], [f32; 4])){
    let voltage_1 = ina260_1.voltage().await;
    let v1_ts = Mono::now().ticks();

    let voltage_2 = ina260_2.voltage().await;
    let v2_ts = Mono::now().ticks();

    let voltage_3 = ina260_3.voltage().await;
    let v3_ts = Mono::now().ticks();

    let voltage_4 = ina260_4.voltage().await;
    let v4_ts = Mono::now().ticks();

    let current_1 = ina260_1.current().await;
    let i1_ts = Mono::now().ticks();

    let current_2 = ina260_2.current().await;
    let i2_ts = Mono::now().ticks();

    let current_3 = ina260_3.current().await;
    let i3_ts = Mono::now().ticks();

    let current_4 = ina260_4.current().await;
    let i4_ts = Mono::now().ticks();

    let power_1 = ina260_1.power().await;
    let p1_ts = Mono::now().ticks();

    let power_2 = ina260_2.power().await;
    let p2_ts = Mono::now().ticks();

    let power_3 = ina260_3.power().await;
    let p3_ts = Mono::now().ticks();

    let power_4 = ina260_4.power().await;
    let p4_ts = Mono::now().ticks();

    let mut voltage_slice = [0.0_f32; 4];
    let mut v_ts_slice = [v1_ts, v2_ts, v3_ts, v4_ts];

    let mut current_slice = [0.0_f32; 4];
    let mut i_ts_slice = [i1_ts, i2_ts, i3_ts, i4_ts];

    let mut power_slice = [0.0_f32; 4];
    let mut p_ts_slice = [p1_ts, p2_ts, p3_ts, p4_ts];

    match voltage_1 {
        Ok(voltage)=>{
            voltage_slice[0] = voltage;
        }
        Err(i2c_error)=>{
            error!("V1 Err: {}", i2c_error);
            voltage_slice[0] = f32::NAN;
        }
    }
    match voltage_2 {
        Ok(voltage)=>{
            voltage_slice[1] = voltage;
        }
        Err(i2c_error)=>{
            error!("V2 Err: {}", i2c_error);
            voltage_slice[1] = f32::NAN;
        }
    }
    match voltage_3 {
        Ok(voltage)=>{
            voltage_slice[2] = voltage;
        }
        Err(i2c_error)=>{
            error!("V3 Err: {}", i2c_error);
            voltage_slice[2] = f32::NAN;
        }
    }
    match voltage_4 {
        Ok(voltage)=>{
            voltage_slice[3] = voltage;
        }
        Err(i2c_error)=>{
            error!("V4 Err: {}", i2c_error);
            voltage_slice[3] = f32::NAN;
        }
    }
    match current_1 {
        Ok(current)=>{
            current_slice[0] = current;
        }
        Err(i2c_error)=>{
            error!("I1 Err: {}", i2c_error);
            current_slice[0] = f32::NAN;
        }
    }
    match current_2 {
        Ok(current)=>{
            current_slice[1] = current;
        }
        Err(i2c_error)=>{
            error!("I2 Err: {}", i2c_error);
            current_slice[1] = f32::NAN;
        }
    }
    match current_3 {
        Ok(current)=>{
            current_slice[2] = current;
        }
        Err(i2c_error)=>{
            error!("I3 Err: {}", i2c_error);
            current_slice[2] = f32::NAN;
        }
    }
    match current_4 {
        Ok(current)=>{
            current_slice[3] = current;
        }
        Err(i2c_error)=>{
            error!("I4 Err: {}", i2c_error);
            current_slice[3] = f32::NAN;
        }
    }
    match power_1 {
        Ok(power)=>{
            power_slice[0] = power;
        }
        Err(i2c_error)=>{
            error!("P1 Err: {}", i2c_error);
            current_slice[0] = f32::NAN;
        }
    }
    match power_2 {
        Ok(power)=>{
            power_slice[1] = power;
        }
        Err(i2c_error)=>{
            error!("P2 Err: {}", i2c_error);
            power_slice[1] = f32::NAN;
        }
    }
    match power_3 {
        Ok(power)=>{
            power_slice[2] = power;
        }
        Err(i2c_error)=>{
            error!("P3 Err: {}", i2c_error);
            power_slice[2] = f32::NAN;
        }
    }
    match power_4 {
        Ok(power)=>{
            power_slice[3] = power;
        }
        Err(i2c_error)=>{
            error!("P4 Err: {}", i2c_error);
            power_slice[3] = f32::NAN;
        }
    }
    let full_slice = ((v_ts_slice, i_ts_slice, p_ts_slice), (voltage_slice, current_slice, power_slice));
    return full_slice;
}