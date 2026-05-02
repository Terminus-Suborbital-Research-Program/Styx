#![warn(missing_docs)]

//! TERMINUS RS-X 2026 Elara JUPITER Code

use std::{
    os::unix::thread, thread::sleep, time::{Duration, Instant}
};

use aether::color;
use avionics::lsm6dsl::Lsm6DslAccel;
use bin_packets::{
    data::status, device::{PacketReader, PacketWriter, std::Device}, packets::ApplicationPacket
};
use common_states::rbf::ActiveHighRbf;
use constants::{EJECTION_IND_PIN, RBF_PIN};
use data::packets::OnboardPacketStorage;
use env_logger::Env;

use gpio::{Pin, read::ReadPin, write::WritePin};
use i2cdev::linux::LinuxI2CDevice;
use states::JupiterStateMachine;
use tasks::{Atmega, spawn_camera_thread, InfratrackerThread};

mod avionics;
mod constants;
mod data;
mod gpio;
mod states;
mod tasks;
mod timing;

use data::status::ExperimentColorState;
use log::{error, info};
use tasks::{RbfTask, GpioHardware, GuardMonitor};
use bin_packets::commands::CommandPacket;

static SERIAL_PORT: &str = "/dev/ttyS0";

fn main() {
    let env = Env::default().filter_or("LOG_LEVEL", "info");
    env_logger::init_from_env(env);

    let startup = Instant::now();

    // Immediantly access POWER_ON_TIME to evaluate the lazy_static
    let _ = timing::POWER_ON_TIME;

    let port_res = serialport::new(SERIAL_PORT, 115200)
        .timeout(Duration::from_millis(10))
        .open();
    
    let mut interface = match port_res {
        Ok(port) => Some(Device::new(port)),
        Err(e) => {
            error!("Failed to open serial port {SERIAL_PORT}: {e}");
            None // Continue booting so we can still log to SD card
        }
    };

    let ejection_pin: WritePin = Pin::new(EJECTION_IND_PIN).into();
    if let Err(e) = ejection_pin.write(false) {
        error!("Failed to set ejection pin low on boot: {:?}", e);
    }

    #[cfg(feature = "legacy_atmega")]
    let hardware = {
        let i2c_device = LinuxI2CDevice::new("/dev/i2c-1", 0x26u16).expect("CRITICAL: Failed Atmega I2C");
        Atmega::new(i2c_device)
    };

    #[cfg(not(feature = "legacy_atmega"))]
    let hardware = GpioHardware::new();

    // Main camera
    spawn_camera_thread();

    let accel = match Lsm6DslAccel::new() {
        Ok(a) => {
            info!("Accelerometer read: {:?}", a.read_data());
            Some(a)
        }
        Err(e) => {
            error!("Failed to initialize accelerometer: {e}");
            None
        }
    };

    let mut onboard_packet_storage = OnboardPacketStorage::get_current_run();

    let (infratracker_thread, infratracker_packet_rx) = InfratrackerThread::new();
    let infratracker_handle = infratracker_thread.begin_startracking();

    let mut state_machine = JupiterStateMachine::new(hardware, ejection_pin);
    let mut counter = 0;

    let mut color_status = ExperimentColorState::new();

    let mut guard_monitor = GuardMonitor::new("/home/terminus/rad_data", 3);

    let mut last_rgb_options = color_status.current_status();
    loop {
        if let Some(iface) = &mut interface {
            while let Some(packet) = iface.read() {
                match &packet {
                    ApplicationPacket::GeigerData { timestamp_ms: _, recorded_pulses: _ } => {
                        color_status.feed_geiger();
                    }
                    ApplicationPacket::ThermocoupleData { timestamp: _, hot_junction_temp: _ }=> {
                        color_status.feed_thermocouple();
                    }
                    _ => {}
                }

                onboard_packet_storage.write(packet); // Write to the onboard storage
                #[cfg(feature = "packet_logging")]
                info!("Got a packet: {packet:?}");
            }
        }

        // Update geiger feed either if we get a geiger packet through serial, or have file update
        guard_monitor.update(&mut color_status);

        while let Ok(quat) = infratracker_packet_rx.try_recv() {
            color_status.feed_infratracker();
            onboard_packet_storage.write(quat); // Write quat to the onboard storage
            #[cfg(feature = "packet_logging")]
            info!("Got a infratracker packet: {quat:?}");
        }

        if let Some(ref a) = accel {
            match a.read_data() {
                Ok(t) => {
                    let packet = ApplicationPacket::JupiterAccelerometer {
                        timestamp_ms: std::time::Instant::now()
                            .duration_since(startup)
                            .as_millis() as u64,
                        vector: t,
                    };
                    color_status.feed_avionics();

                    onboard_packet_storage.write(packet);
                    if let Some(iface) = &mut interface {
                        iface.write(packet).ok();
                    }
                }
                Err(e) => {
                    error!("Issue with the accelerometer: {e:?}");
                }
            }
        }

        state_machine.update();
        color_status.feed_jupiter_state_machine(state_machine.phase());

        let rgb_options = color_status.current_status();
        let current_rgb_options = color_status.current_status();

        // Send new rgb colors on state change
        if current_rgb_options != last_rgb_options {
            if let Some(iface) = &mut interface {
                if let Err(e) = iface.write(ApplicationPacket::Command(CommandPacket::ColorSet(current_rgb_options))) {
                    error!("Failed to write color packet down: {e}");
                }
            }
            last_rgb_options = current_rgb_options;
        }

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