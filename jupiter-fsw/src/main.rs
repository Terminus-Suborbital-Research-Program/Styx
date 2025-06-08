use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use avionics::lsm6dsl::Lsm6DslAccel;
use bin_packets::{
    device::{PacketReader, PacketWriter, std::Device},
    packets::ApplicationPacket,
};
use common::rbf::ActiveHighRbf;
use constants::{EJECTION_IND_PIN, RBF_PIN};
use data::packets::OnboardPacketStorage;
use env_logger::Env;

use gpio::{Pin, read::ReadPin, write::WritePin};
use i2cdev::linux::LinuxI2CDevice;
use states::JupiterStateMachine;
use tasks::{Atmega, spawn_camera_thread};

mod avionics;
mod constants;
mod data;
mod gpio;
mod states;
mod tasks;
mod timing;

use log::{error, info};
use tasks::RbfTask;

static SERIAL_PORT: &str = "/dev/ttyS0";

fn main() {
    let env = Env::default().filter_or("LOG_LEVEL", "info");
    env_logger::init_from_env(env);

    let startup = Instant::now();

    // Immediantly access POWER_ON_TIME to evaluate the lazy_static
    let _ = timing::POWER_ON_TIME;

    let port = serialport::new(SERIAL_PORT, 115200)
        .timeout(Duration::from_millis(10))
        .open()
        .unwrap();
    let mut interface = Device::new(port);
    let rbf_pin = ActiveHighRbf::new(ReadPin::from(Pin::new(RBF_PIN)));
    let ejection_pin: WritePin = Pin::new(EJECTION_IND_PIN).into();
    ejection_pin.write(false).unwrap();

    let atmega = Atmega::new(LinuxI2CDevice::new("/dev/i2c-1", 0x26u16).unwrap());

    let rbf = RbfTask::new(rbf_pin).spawn(100);

    // Main camera
    spawn_camera_thread();

    // Get accelerometer
    let accel = Lsm6DslAccel::new().unwrap();
    info!("Accelerometer read: {:?}", accel.read_data());

    let mut onboard_packet_storage = OnboardPacketStorage::get_current_run();

    info!("RBF At Boot: {}", rbf.read());

    let mut state_machine = JupiterStateMachine::new(atmega, ejection_pin, rbf.clone());
    let mut counter = 0;

    loop {
        while let Some(packet) = interface.read() {
            onboard_packet_storage.write(packet); // Write to the onboard storage
            if let Err(e) = interface.write(packet) {
                error!("Failed to write packet down: {e}");
            }
            info!("Got a packet: {packet:?}");
        }

        match accel.read_data() {
            Ok(t) => {
                let packet = ApplicationPacket::JupiterAccelerometer {
                    timestamp_ms: std::time::Instant::now()
                        .duration_since(startup)
                        .as_millis() as u64,
                    vector: t,
                };

                onboard_packet_storage.write(packet);
                interface.write(packet).ok();
            }
            Err(e) => {
                error!("Issue with the accelerometer: {e:?}");
            }
        }

        state_machine.update();
        if counter % 10 == 0 {
            info!(
                "T{}: {:#?}",
                timing::t_time_estimate(),
                state_machine.phase()
            );
        }
        counter += 1;
        sleep(Duration::from_millis(100));
    }
}
