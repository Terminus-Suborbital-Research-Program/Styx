use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::{sdr::radio_config::RadioConfig, signal::signal_config::SignalConfig};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, default_value_t = 101.1e6)]
    pub frequency: f64,

    #[arg(short, long, default_value_t = 3.0e6)]
    pub sample_rate: f64,

    #[arg(short, long, default_value_t = 64)]
    pub down_size: usize,

    #[arg(long, default_value_t = 100)]
    pub search_size: usize,

    #[arg(short, long, default_value = "capture.iq")]
    capture_output: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn get_configs() -> (RadioConfig, SignalConfig) {
        let cli = Cli::parse();
        let radio_config = RadioConfig::new(cli.frequency, cli.sample_rate);
        let signal_config = SignalConfig::new(cli.capture_output, cli.down_size, cli.search_size);
        (radio_config, signal_config)
    }

    pub fn run_commands() -> (bool, PathBuf) {
        let cli = Cli::parse();
        match &cli.command {
            Commands::Capture { output } => {
                println!("Saving baseline to: {:?}", output);
                let record_baseline = true;
                let psd_path = output.clone();
                (record_baseline, psd_path)
            }
            Commands::Compare { input } => {
                println!("Loading baseline from: {:?}", input);
                let record_baseline = false;
                let psd_path = input.clone();
                (record_baseline, psd_path)
            }
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Captures a single packet and saves the PSD baseline
    Capture {
        /// Where to save the baseline PSD file
        #[arg(short, long, default_value = "comp.psd")]
        output: PathBuf,
    },
    /// Runs the SDR and compares live signals against the baseline
    Compare {
        /// The baseline PSD file to load
        #[arg(short, long, default_value = "comp.psd")]
        input: PathBuf,
    },
}
