#![warn(missing_docs)]

use crate::{ElaraTests, Protocol, cli::bincode_serialize, serial};
use chrono::{DateTime, Local, NaiveTime, TimeDelta};
use clap::Subcommand;
use std::{ops::Sub, path::PathBuf};

#[derive(Subcommand)]
pub enum Commands {
    /// Command to listen to a serial device
    #[command(name = "listen")]
    SerailListen {
        

        #[arg(help = "Integer for the baud_rate.")]
        baud_rate: u32,
    },

    SshListen {},

    /// Command to list all serial devices connected to the device
    #[command(name = "list")]
    List {
        #[arg(help = "")]
        filter: Option<String>,
    },

    /// Command to send a test packet to a device
    #[command(name = "test")]
    Test {
        #[arg(help = "")]
        test: Option<ElaraTests>,
    },

    #[command(name = "version")]
    Version {},
}

impl Commands {
    pub fn execute(
        self,
        file: Option<PathBuf>,
        protocol: Option<Protocol>,
        test_: Option<ElaraTests>,
        device_p: Option<String>
    ) -> () {
        match self {
            Commands::SerailListen {baud_rate } => {
                serial::USB::read_serial(&device_p.expect("You must porvide a serial device path"), baud_rate);
            }
            Commands::List { filter } => {
                serial::USB::list_all_devices();
            }
            Commands::Test { test } => match test_ {
                Some(test_packet) => {
                    println!("Running Test: {:?}", test_packet);
bincode_serialize::send_packet(device_p.expect("Serial device path is needed").as_str(), 9600, test_packet);                }
                None => {
                    println!("No test provided.");
                }
            },
            Commands::SshListen {} => {
                println!("Not implemented yet!");
            }
            Commands::Version {} => {
                println!("{}: {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            }
        }
    }
}

/// Struct for
pub struct TimeoutTimer {
    last_recieve_time: NaiveTime,
    timeout_duriation: TimeDelta,
}

impl TimeoutTimer {
    /// Creates a Timer with a timeout_duration of 5 minutes
    pub fn new() -> Self {
        return Self {
            last_recieve_time: Local::now().time(),
            timeout_duriation: TimeDelta::seconds(10),
        };
    }

    /// Updates the timeout_duration to a new value
    pub fn set_duration(&mut self, new_duration: i64) -> &mut Self {
        self.timeout_duriation = TimeDelta::seconds(new_duration);
        return self;
    }

    /// Updates the time since the last packet
    pub fn update_time(&mut self) -> () {
        self.last_recieve_time = Local::now().time();
    }

    /// Checks if a set ammount of time has passed since a packet was recieved
    pub fn timeout_check(&self) -> bool {
        if self.last_recieve_time.sub(Local::now().time()) >= self.timeout_duriation {
            return true;
        }
        return false;
    }
}
