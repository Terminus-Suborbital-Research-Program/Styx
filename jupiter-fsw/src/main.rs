use std::{
    sync::{Arc, RwLock},
    thread::{self, sleep},
    time::Duration,
};

use bin_packets::device::PacketIO;
use bin_packets::{device::PacketDevice, phases::JupiterPhase};
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

use crate::db::db_init;
use log::info;

static SERIAL_PORT: &str = "/dev/serial0";

fn main() {
    let env = Env::default().filter_or("LOG_LEVEL", "info");
    env_logger::init_from_env(env);

    let port = serialport::new(SERIAL_PORT, 115200)
        .timeout(Duration::from_millis(10))
        .open()
        .unwrap();
    let mut interface = PacketDevice::new(port);
    let rbf_pin: RbfPin = ReadPin::from(Pin::new(RBF_PIN)).into();
    let ejection_pin: WritePin = Pin::new(EJECTION_IND_PIN).into();
    ejection_pin.write(true).unwrap();

    let mut atmega = LinuxI2CDevice::new("/dev/i2c-1", 0x26u16).unwrap();

    info!("I2c Read: {:?}", atmega.smbus_read_byte());
    let states = Arc::new(RwLock::new(PinStates::default()));

    let state_writer = Arc::clone(&states);
    let pin_state_state_machine = Arc::clone(&states);

    let mut state_machine = JupiterStateMachine::new(pin_state_state_machine);

    thread::spawn(move || {
        pin_states_thread(atmega, state_writer);
    });

    db_init();

    thread::spawn(move || ping_thread());

    info!("Current Iteration: {}", db::current_iteration_num());

    loop {
        interface.update().ok();

        info!("Top packet: {:?}", interface.read_packet());

        let transition = state_machine.update();

        if transition.is_some() {
            let new_state = transition.unwrap();
            info!("New State: {:?}", new_state);

            match new_state {
                JupiterPhase::PowerOn => {
                    info!("Yippee!");
                }

                JupiterPhase::MainCamStart => {
                    if !rbf_pin.is_inserted() {
                        info!("Main cam starting...");
                    } else {
                        info!("RBF inserted, inhibiting video, to save space.");
                    }
                }

                JupiterPhase::Launch => {
                    info!("Hold on to your hats!");
                }

                JupiterPhase::SecondaryCamStart => {
                    if !rbf_pin.is_inserted() {
                        info!("Secondary cam starting...");
                    } else {
                        info!("RBF inserted, inhibiting ESP-32 startup.");
                    }
                }

                JupiterPhase::SkirtEjection => {
                    if !rbf_pin.is_inserted() {
                        info!("GET OUTTA HERE!");
                        ejection_pin.write(true).unwrap();
                    } else {
                        info!("No actions to take.");
                    }
                }

                JupiterPhase::BatteryPower => {
                    info!("Battery power on.");
                }

                JupiterPhase::Shutdown => {
                    info!("Shutting down...");
                }
            }
        }

        sleep(Duration::from_millis(1000));
    }
}
