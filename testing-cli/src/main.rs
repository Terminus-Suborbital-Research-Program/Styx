#![warn(missing_docs)]

//! d

pub mod cli;

use std::path::PathBuf;

use clap::Parser; //#use rusb::{self, DeviceDescriptor};
use serialport::{SerialPortType, available_ports};

use bin_packets::packets::{ApplicationPacket, testing::*};

use crate::cli::{
    commands::*,
    enums::*,
    serial::{self},
};

#[derive(Parser)]
#[command(name = "test_embedded")]
struct TestCli {
    #[command(subcommand)]
    command: Commands,

    #[arg(
        short = 'P',
        help = "Used to specify what what protocol to use for a command",
        required = false
    )]
    protocol: Option<Protocol>,

    #[arg(short = 'T', help = "Which test to run", required = false)]
    test: Option<ElaraTests>,

    #[arg(
        short = 'F',
        help = "Used to pipe output to a file for a command",
        required = false
    )]
    file: Option<PathBuf>,

    #[arg(
        short = 'D',
        help = "Used to set a device for a command",
        required = false
    )]
    device_path: Option<String>,
}

fn main() {
    let cli_command = TestCli::parse();
    let file_ = cli_command.file;
    let protocol_ = cli_command.protocol;
    let test_ = cli_command.test;
    let device_ = cli_command.device_path;

    cli_command.command.execute(file_, protocol_, test_, device_);
    //serial::USB::read_serial("/dev/ttyACM1", 9600);
}
