mod csv_translator;
mod parser;
mod parser_builder;
use crate::parser_builder::DataParserBuilder;

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "data")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Read a bincode file to terminal
    Read {
        #[arg(help = "Raw binary file to convert")]
        read_file_path: String,
        #[arg(long, short, help = "Output to file flag")]
        output: bool,
        #[arg(help = "Path for the directory to save readable data (folder, not file)")]
        write_file_path: Option<String>,
    },

    Write {
        #[arg(help = "Raw binary file to convert")]
        read_file_path: String,
        #[arg(help = "Path for the directory to save readable data (folder, not file)")]
        write_file_path: String,
        #[arg(long, short, help = "Date and Timestamp the File")]
        time: bool,
        #[arg(long, short, help = "Iterate the filename or overwrite?")]
        iterate: bool,
    },
}

impl Commands {
    fn execute(self) {
        match self {
            Commands::Read {
                read_file_path,
                output,
                write_file_path,
            } => {
                let data_parser = DataParserBuilder::new()
                    .write_to_file(output, PathBuf::from(write_file_path.unwrap()))
                    .write_to_stdout(true)
                    .build();

                data_parser.parse_file(Path::new(&read_file_path));
            }
            // Self::read(read_file_path, true, output, write_file_path),
            Commands::Write {
                read_file_path,
                write_file_path,
                time,
                iterate,
            } => {
                let data_parser = DataParserBuilder::new()
                    .write_to_file(true, PathBuf::from(write_file_path))
                    .write_to_stdout(false)
                    .iterate(iterate)
                    .time(time)
                    .build();

                data_parser.parse_file(Path::new(&read_file_path));
            }
        }
    }
}
fn main() {
    let cli = Cli::parse();
    cli.command.execute();
}
