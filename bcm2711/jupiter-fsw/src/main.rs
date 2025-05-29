use std::{thread::sleep, time::Duration};

use bin_packets::device::{PacketReader, PacketWriter, std::Device};
use common::rbf::ActiveHighRbf;
use constants::{EJECTION_IND_PIN, RBF_PIN};
use data::packets::OnboardPacketStorage;
use env_logger::Env;

use gpio::{Pin, read::ReadPin, write::WritePin};
use i2cdev::{core::I2CDevice, linux::LinuxI2CDevice};
use states::JupiterStateMachine;
use tasks::{Atmega, IndicatorsReader, spawn_camera_thread};

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

    let mut atmega = LinuxI2CDevice::new("/dev/i2c-1", 0x26u16).unwrap();

    info!("I2c Read: {:?}", atmega.smbus_read_byte());

    let rbf = RbfTask::new(rbf_pin).spawn(100);

    // Main camera
    spawn_camera_thread();

    let mut onboard_packet_storage = OnboardPacketStorage::get_current_run();

    info!("RBF At Boot: {}", rbf.read());

    let mut state_machine = JupiterStateMachine::new(
        Atmega::new(LinuxI2CDevice::new("/dev/i2c-1", 0x26u16).unwrap()),
        ejection_pin,
        rbf.clone(),
    );

    loop {
        while let Some(packet) = interface.read() {
            onboard_packet_storage.write(packet); // Write to the onboard storage
            if let Err(e) = interface.write(packet) {
                error!("Failed to write packet down: {}", e);
            }
            info!("Got a packet: {:?}", packet);
        }

        state_machine.update();

        info!(
            "T{}: {:#?}",
            timing::t_time_estimate(),
            state_machine.phase()
        );
        sleep(Duration::from_millis(1000));
    }
}
