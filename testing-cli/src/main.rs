#![warn(missing_docs)]

//! d

pub mod cli;

use std::path::PathBuf;

use clap::Parser;//#use rusb::{self, DeviceDescriptor};
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

    #[arg(
        short = 'T',
        help = "Which test to run", 
        required = false
    )]
    test: Option<Tests>,

    #[arg(
        short = 'F',
        help = "Used to pipe output to a file for a command",
        required = false
    )]
    file : Option<PathBuf>
}

fn main() {
    let cli_command = TestCli::parse();
    cli_command.command.execute();

    //serial::USB::read_serial("/dev/ttyACM1", 9600);
}
