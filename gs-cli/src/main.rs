use std::time::Duration;

use bin_packets::packets::ApplicationPacket;
use bincode::{config::standard, decode_from_slice};
use clap::{Parser, Subcommand};
use serialport::SerialPortType;

#[derive(Parser)]
#[command(name = "groundstation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(name = "listen")]
    /// Listens for incoming packets on a serial port
    Listen {
        #[arg(help = "Serial port to listen on")]
        port: String,
    },
    /// Lists all available serial ports
    List,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Listen { port } => listen(&port),

        Commands::List => {
            let ports = serialport::available_ports().expect("Failed to list serial ports");
            ports.iter().for_each(|port| match &port.port_type {
                SerialPortType::UsbPort(_) => {
                    println!("USB Port: {}", port.port_name);
                }

                SerialPortType::Unknown => {
                    // Nothing
                }

                SerialPortType::BluetoothPort => {
                    println!("Bluetooth Port: {} - {:?}", port.port_name, port.port_type);
                }

                SerialPortType::PciPort => {
                    println!("PCI Port: {} - {:?}", port.port_name, port.port_type);
                }
            });
        }
    }
}

fn listen(port: &str) -> ! {
    let mut port = serialport::new(port, 9600)
        .timeout(Duration::from_millis(10))
        .open()
        .expect("Failed to open seial port!");

    let mut serial_buf: Vec<u8> = vec![0; 256];
    let mut incoming_chars: Vec<u8> = Vec::new();
    loop {
        match port.read(serial_buf.as_mut_slice()) {
            Ok(t) => {
                incoming_chars.extend_from_slice(&serial_buf[..t]);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }

        while !incoming_chars.is_empty() {
            match decode_from_slice::<ApplicationPacket, _>(&incoming_chars, standard()) {
                Ok((packet, consumed)) => {
                    println!("{:?}", packet);
                    incoming_chars = incoming_chars.split_off(consumed);
                }
                #[allow(unused_variables)]
                Err(bincode::error::DecodeError::UnexpectedEnd { additional }) => {
                    incoming_chars.remove(0);
                }

                Err(e) => {
                    eprintln!("{:?}", e);
                    incoming_chars.clear();
                }
            }
        }
    }
}
