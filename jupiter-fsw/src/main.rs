use std::{
    sync::{Arc, RwLock},
    thread::{self, sleep},
    time::Duration,
};

use bin_packets::device::PacketIO;
use bin_packets::device::PacketDevice;
use constants::{EJECTION_IND_PIN, RBF_PIN};
use env_logger::Env;

use gpio::{Pin, read::ReadPin, write::WritePin};
use i2cdev::{core::I2CDevice, linux::LinuxI2CDevice};
use palantir::ping_thread;
use rbf::RbfPin;
use states::JupiterStateMachine;
use tasks::{PinStates, pin_states_thread};

mod constants;
mod db;
mod gpio;
mod palantir;
mod rbf;
mod states;
mod tasks;
mod timing;

use crate::db::db_init;
use log::info;

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
    let _rbf_pin: RbfPin = ReadPin::from(Pin::new(RBF_PIN)).into();
    let ejection_pin: WritePin = Pin::new(EJECTION_IND_PIN).into();
    ejection_pin.write(true).unwrap();

    let mut atmega = LinuxI2CDevice::new("/dev/i2c-1", 0x26u16).unwrap();

    info!("I2c Read: {:?}", atmega.smbus_read_byte());
    let states = Arc::new(RwLock::new(PinStates::default()));

    let state_writer = Arc::clone(&states);
    let pin_arc = Arc::clone(&states);

    let mut state_machine = JupiterStateMachine::new(pin_arc);

    thread::spawn(move || {
        pin_states_thread(atmega, state_writer);
    });

    db_init();

    thread::spawn(move || ping_thread());

    info!("Current Iteration: {}", db::current_iteration_num());

    loop {
        interface.update().ok();

        state_machine.update();

        info!("T+: {}", timing::t_time_estimate());
        sleep(Duration::from_millis(1000));
    }
}
