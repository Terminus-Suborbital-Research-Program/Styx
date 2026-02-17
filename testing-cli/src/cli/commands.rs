#![warn(missing_docs)]

use crate::{Protocol, Tests, serial};
use clap::Subcommand;
use std::{ops::Sub, path::PathBuf};
use chrono::{DateTime, Local, NaiveTime, TimeDelta};

#[derive(Subcommand)]
pub enum Commands {

    /// Command to listen to a serial device
    #[command(name = "listen")]
    SerailListen {
        #[arg(
            help = "Device path. Does not accept partial names."
        )]
        device: String,

        #[arg(
            help = "Integer for the baud_rate."
        )]
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
        test: Tests,
    },


}

impl Commands {
    pub fn execute(self) -> () {
        match self {
            Commands::SerailListen { device, baud_rate } => {
                serial::USB::read_serial(&device, baud_rate);
            }
            Commands::List { filter } => {
                    serial::USB::list_all_devices();
            }
            Commands::Test { test } => {
                println!("{:?}", test);
            }
            Commands::SshListen {  } => {
                println!("Not implemented yet!")
            },
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

    /// cChecks if a set ammount of time has passed since a packet was recieved
    pub fn timeout_check(&self) -> bool {
        if self.last_recieve_time.sub(Local::now().time()) >= self.timeout_duriation {
            return true;
        }
        return false;
    }
}
