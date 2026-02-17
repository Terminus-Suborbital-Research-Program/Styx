#![warn(missing_docs, redundant_imports, redundant_semicolons)]

use std::{
    io::{self, Write},
    iter,
    mem::MaybeUninit,
    usize,
};

use bincode::de;

use serialport::{SerialPortInfo, SerialPortType, available_ports};

use crate::cli::serial;

/// Struct used for handeling USB devices
pub struct USB {}

/// Tuple type for holding Device Date together
pub type USBDeviceData = (u8, u8, u16, u16, String, String);

impl USB {
    /// Function to get USB device data
    /// BUG: Currently getting the manufacuter &
    /// product strings requires sudo
    pub fn get_device_info() -> USBDeviceData {
        let mut results: (u8, u8, u16, u16, String, String) =
            (0, 0, 0, 0, "".to_owned(), "".to_owned());

        results
    }

    /// Function to list all usb devices
    pub fn list_all_devices() -> () {
        let mut ports = serial::USB::get_all_devices();
        for port in ports {
            match port.port_type {
                SerialPortType::UsbPort(info) => {
                    println!("Path: {}", port.port_name);
                    println!("Type: USB");
                    println!("VID: {:04x}", info.vid);
                    println!("PID: {:04x}", info.pid);
                    #[cfg(feature = "usbportinfo-interface")]
                    println!(
                        "        Interface: {}",
                        info.interface
                            .as_ref()
                            .map_or("".to_string(), |x| format!("{:02x}", *x))
                    );
                    println!(
                        "Serial Number: {}",
                        info.serial_number.as_ref().map_or("", String::as_str)
                    );
                    println!(
                        "Manufacturer: {}",
                        info.manufacturer.as_ref().map_or("", String::as_str)
                    );
                    println!(
                        "Product: {}",
                        info.product.as_ref().map_or("", String::as_str)
                    );
                }
                _ => {}
            }
        }
    }

    /// Function to return a vector of Devices and the devices' info
    pub fn get_all_devices() -> Vec<SerialPortInfo> {
        return available_ports().expect("Failed");
    }

    /// Write a buffer into a USB Device
    /// Returns the number of bytes if successful
    /// or usize:MAX if unsucessful since it's
    /// unlikely that the you ever tranfer 2^64 bytes of data


    pub fn read_serial(port_path: &str, baud_rate: u32) -> () {
        let port = serialport::new(port_path, baud_rate)
            .timeout(std::time::Duration::from_millis(100))
            .open();
        match port {
            Ok(mut port) => {
                let mut serial_buf: Vec<u8> = vec![0; 1000];
                println!("Receiving data on {} at {} baud:", &port_path, &baud_rate);
                loop {
                    match port.read(serial_buf.as_mut_slice()) {
                        Ok(t) => {
                            io::stdout().write_all(&serial_buf[..t]).unwrap();
                            io::stdout().flush().unwrap();
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to open \"{}\". Error: {}", port_path, e);
                std::process::exit(1);
            }
        }
    }
    fn valid_baud(val: &str) -> Result<(), String> {
        val.parse::<u32>()
            .map(|_| ())
            .map_err(|_| format!("Invalid baud rate '{}' specified", val))
    }
}
