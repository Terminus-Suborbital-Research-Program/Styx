#![warn(missing_docs)]

//! TERMINUS RS-X 2026 Elara JUPITER Code

use std::{
    os::unix::thread, thread::sleep, time::{Duration, Instant},
    net::{TcpListener, TcpStream}
};
use std::fs::OpenOptions;
use std::io::{BufWriter, Read, Write};
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
use std::sync::atomic::{AtomicBool, Ordering};

use signet::sdr::radio_config::BUFF_SIZE;

mod avionics;
mod constants;
mod data;
mod gpio;
mod states;
mod tasks;
mod timing;

use data::status::ExperimentColorState;
use log::{error, info};
use tasks::{RbfTask, GpioHardware, LogMonitor, TRACKING};
use bin_packets::commands::CommandPacket;
use avionics::imu::{AvionicsImuManager, IMUError};


// pub const CAM_ON_PIN: &str = "GPIO18"; // G3

static SERIAL_PORT: &str = "/dev/ttyS0";

pub const STATUS_INTERVAL: u64 = 1000;

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
    
    // let cam_pin: WritePin = Pin::new(CAM_ON_PIN).into();
    //  if let Err(e) = cam_pin.write(true) {
    //     error!("Failed to set ejection pin low on boot: {:?}", e);
    // }    


    #[cfg(feature = "legacy_atmega")]
    let hardware = {
        let i2c_device = LinuxI2CDevice::new("/dev/i2c-1", 0x26u16).expect("CRITICAL: Failed Atmega I2C");
        Atmega::new(i2c_device)
    };

    #[cfg(not(feature = "legacy_atmega"))]
    let hardware = GpioHardware::new();

    // Main camera
    // spawn_camera_thread();
    // TRACKING.store(true, Ordering::Relaxed);

    let mut avionics = match AvionicsImuManager::new() {
        Ok(manager) => {
            info!("Avionics IMU Manager initialized successfully.");
            Some(manager)
        }
        Err(e) => {
            match e {
                IMUError::BusFailed(i2c_err) => error!("IMU Init Failed (I2C Bus Error): {:?}", i2c_err),
                IMUError::SensorFailed(adxl_err) => error!("IMU Init Failed (ADXL375 Error): {:?}", adxl_err),
                IMUError::BMIFail(bmi_err) => error!("IMU Init Failed (BMI323 Error): {:?}", bmi_err),
            }
            None
        }
    };

    let mut onboard_packet_storage = OnboardPacketStorage::get_current_run();

    let (infratracker_thread, infratracker_packet_rx) = InfratrackerThread::new();
    let infratracker_handle = infratracker_thread.begin_startracking();

    let mut state_machine = JupiterStateMachine::new(hardware, ejection_pin);
    let mut counter = 0;

    let mut color_status = ExperimentColorState::new();

    let mut guard_monitor = LogMonitor::new("/home/terminus/rad_data", 3);

    let mut infratracker_monitor = LogMonitor::new("/home/terminus/basler", 3);


    let mut last_rgb_options = color_status.current_status();

    let mut last_update = Instant::now();
    let status_interval = Duration::from_millis(STATUS_INTERVAL);


    loop {
        if let Some(iface) = &mut interface {
            while let Some(packet) = iface.read() {
                match &packet {
                    ApplicationPacket::GeigerData { timestamp_ms: _, recorded_pulses: _ } => {
                        color_status.feed_geiger();
                    }
                    ApplicationPacket::ThermocoupleData { timestamp: _, channel: _, hot_junction_temp: _ }=> {
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
        if guard_monitor.is_updated() {
            color_status.feed_geiger();
        }


        if infratracker_monitor.is_updated() {
            color_status.feed_infratracker();
        }



        while let Ok(quat) = infratracker_packet_rx.try_recv() {
            // info!("Infratracker alive");

            onboard_packet_storage.write(quat); // Write quat to the onboard storage
            #[cfg(feature = "packet_logging")]
            info!("Got a infratracker packet: {quat:?}");
        }

        if let Some(ref mut a) = avionics {
            let imu_data = a.read_all(startup);
            let mut imu_alive = false;

            // if let Some(packet) = imu_data.high_range {
            //     imu_alive = true;
            //     onboard_packet_storage.write(packet.clone());
                
            //     #[cfg(feature = "packet_logging")]
            //     info!("Got High-G Accel packet: {packet:?}");
            // } else {
            //     #[cfg(feature = "packet_logging")]
            //     error!("Read failure: High-G (ADXL375) missing from read_all");
            // }

            if let Some(packet) = imu_data.low_range {
                imu_alive = true;
                onboard_packet_storage.write(packet.clone());
                
                #[cfg(feature = "packet_logging")]
                info!("Got Low-G Accel packet: {packet:?}");
            } else {
                #[cfg(feature = "packet_logging")]
                error!("Read failure: Low-G (BMI323) Accel missing from read_all");
            }

            if let Some(packet) = imu_data.gyro {
                imu_alive = true;
                onboard_packet_storage.write(packet.clone());
                
                #[cfg(feature = "packet_logging")]
                info!("Got Low-G Gyro packet: {packet:?}");
            } else {
                #[cfg(feature = "packet_logging")]
                error!("Read failure: Low-G (BMI323) Gyro missing from read_all");
            }

            // If any data gotten from IMU's, update health
            if imu_alive {
                // info!("Avionics alive");
                color_status.feed_avionics();
            }
        }

        // JUPITER-ODIN Packet Comms
        let sdr_listener = TcpListener::bind("127.0.0.1:7878").expect("Failed to bind");
        info!("Recorder listening on 127.0.0.1:7878...");

        let adcs_listener = TcpListener::bind("127.0.0.2:7878").expect("Failed to bind");
        info!("Recorder listening on 127.0.0.2:7878...");

        let mut sdr_buffer = [0u8; BUFF_SIZE * 10];
        let mut adcs_buffer = [0u8; 1000];

        for stream in sdr_listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    info!("Connection established: {:?}", stream.peer_addr());

                    loop {
                        match stream.read(&mut sdr_buffer) {
                            Ok(0) => {
                                info!("Sender disconnected. Closing file.");
                                break;
                            }
                            Ok(bytes_read) => {
                                if let Err(e) = onboard_packet_storage.write(&sdr_buffer[..bytes_read]) {
                                    error!("Error writing encoded data {}", e);
                                }
                            }

                            Err(e) => {
                                error!("Error reading from socket{}", e);
                            }
                        }
                    }
                }
                Err(e) => error!("Connection failed: {}", e),
            }
        }

        for stream in adcs_listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    info!("Connection established: {:?}", stream.peer_addr());

                    loop {
                        match stream.read(&mut adcs_buffer) {
                            Ok(0) => {
                                info!("Sender disconnected. Closing file.");
                                break;
                            }
                            Ok(bytes_read) => {
                                if let Err(e) = onboard_packet_storage.write(&adcs_buffer[..bytes_read]) {
                                    error!("Error writing encoded data {}", e);
                                }
                            }

                            Err(e) => {
                                error!("Error reading from socket{}", e);
                            }
                        }
                    }
                }
                Err(e) => error!("Connection failed: {}", e),
            }
        }
        ////

        state_machine.update();
        color_status.feed_jupiter_state_machine(state_machine.phase());

        let rgb_options = color_status.current_status();

        let now = Instant::now();

        // Send new rgb colors on state change
        if now.duration_since(last_update) >= status_interval {
            let current_rgb_options = color_status.current_status();

            // info!("Status update");
            if let Some(iface) = &mut interface {
                if let Err(e) = iface.write(ApplicationPacket::Command(CommandPacket::ColorSet(current_rgb_options))) {
                    error!("Failed to write color packet down: {e}");
                }
            }
            last_update = now;
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