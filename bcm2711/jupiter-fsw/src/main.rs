use std::{
    thread::sleep,
    time::Duration,
};

use bin_packets::device::PacketDevice;
use bin_packets::device::PacketIO;
use common::rbf::ActiveHighRbf;
use constants::{EJECTION_IND_PIN, RBF_PIN};
use env_logger::Env;

use gpio::{Pin, read::ReadPin, write::WritePin};
use i2cdev::{core::I2CDevice, linux::LinuxI2CDevice};
use states::JupiterStateMachine;
use tasks::IndicatorsReader;

mod constants;
mod gpio;
mod states;
mod tasks;
mod timing;

use log::info;
use tasks::RbfTask;

static SERIAL_PORT: &str = "/dev/serial0";

fn main() {
    let env = Env::default().filter_or("LOG_LEVEL", "info");
    env_logger::init_from_env(env);

    // Immediantly access POWER_ON_TIME to evaluate the lazy_static
    let _ = timing::POWER_ON_TIME;

    let port = serialport::new(SERIAL_PORT, 115200)
        .timeout(Duration::from_millis(10))
        .open()
        .unwrap();
    let mut interface = PacketDevice::new(port);
    let rbf_pin = ActiveHighRbf::new(ReadPin::from(Pin::new(RBF_PIN)));
    let ejection_pin: WritePin = Pin::new(EJECTION_IND_PIN).into();
    ejection_pin.write(false).unwrap();

    let mut atmega = LinuxI2CDevice::new("/dev/i2c-1", 0x26u16).unwrap();

    info!("I2c Read: {:?}", atmega.smbus_read_byte());

    let pins = IndicatorsReader::new(atmega);
    let rbf = RbfTask::new(rbf_pin).spawn(100);

    info!("RBF At Boot: {}", rbf.read());

    let mut state_machine = JupiterStateMachine::new(pins, ejection_pin);

    loop {
        interface.update().ok();

        state_machine.update();

        info!(
            "T{}: {:#?}",
            timing::t_time_estimate(),
            state_machine.phase()
        );
        sleep(Duration::from_millis(1000));
    }
}
