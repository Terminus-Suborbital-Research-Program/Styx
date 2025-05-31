use clap::{Parser, Subcommand};
use bincode::{config::standard, decode_from_std_read};
use bin_packets::packets::ApplicationPacket;

use std::{fs::{OpenOptions, File}, io::{BufReader, BufWriter, ErrorKind, Read, Write}};

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
        #[arg(long,short,help = "Output to file flag")]
        output: bool,   
        #[arg(help = "Path for file of readable data")]
        write_file_path: Option<String>,
    },

    Write {
        #[arg(help = "Raw binary file to convert")]
        read_file_path: String, 
        #[arg(help = "Path for file of readable data")]
        write_file_path: String,
    }
}

impl Commands {
    fn execute(self) {
        match self {
            Commands::Read { read_file_path , output, write_file_path} => {
                Self::read(read_file_path , true, output, write_file_path)
            }
            Commands::Write { read_file_path, write_file_path } => {
                Self::read(read_file_path, false, true, Some(write_file_path))
            }
        }
    }

    fn read(read_file_path: String, stdout:bool, output: bool, write_file_path: Option<String>) {
        let file = File::open(&read_file_path);

            match file {
                Ok(file) => {
                    let mut reader = BufReader::new(file);

                    let mut writer: Option<BufWriter<File>> = None;
                    if output {
                        if let Some(output_path) = write_file_path {
                            let output_file = OpenOptions::new()
                                                    .create(true)
                                                    .append(true)
                                                    .open(output_path)
                                                    .expect("Uh oh, output file couldn't open");
                            writer = Some(BufWriter::new(output_file));
                        }
                    }
                    loop {
                        let data: Result<ApplicationPacket, bincode::error::DecodeError> = decode_from_std_read(&mut reader, standard());
                        
                        match data {
                            Ok(packet) => {
                                if stdout {
                                    println!("{:#?}", packet);
                                }
                                if output {
                                    if let Some(ref mut file_writer) = writer {
                                        Commands::file_write(packet, file_writer);
                                    }
                                }
                            }
                            Err(e) => {
                                match e {
                                    bincode::error::DecodeError::Io { inner, .. } => {
                                        if inner.kind() == ErrorKind::UnexpectedEof {
                                            break
                                        }
                                    }
                                    _ => eprintln!("Nooo error {}", e),
                                }
                            }
                        }

                    }
                    

                }

                Err(e) => {
                    eprintln!("Error reading raw data from file: {}", e)
                }
        }
    }

    fn file_write(packet: ApplicationPacket, file_writer: &mut BufWriter<File>) {
        if let Err(e) = writeln!(file_writer, "{:#?}", packet ) {
            eprintln!("Error appending to output file: {}", e)
        }
    }
}
fn main() {
    let cli = Cli::parse();
    cli.command.execute();
}
