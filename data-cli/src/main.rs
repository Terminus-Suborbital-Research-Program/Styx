use bin_packets::packets::ApplicationPacket;
use bincode::{config::standard, decode_from_std_read};
use clap::{Parser, Subcommand};
use csv::Writer;
use indexmap::IndexMap;


use std::{
    collections::HashMap, fs::{File, OpenOptions}, io::{BufReader, BufWriter, ErrorKind, Read, Write}
};

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
        #[arg(help = "Path for file of readable data")]
        write_file_path: Option<String>,
    },

    Write {
        #[arg(help = "Raw binary file to convert")]
        read_file_path: String,
        #[arg(help = "Path for file of readable data")]
        write_file_path: String,
    },
}

impl Commands {
    fn execute(self) {
        match self {
            Commands::Read {
                read_file_path,
                output,
                write_file_path,
            } => Self::read(read_file_path, true, output, write_file_path),
            Commands::Write {
                read_file_path,
                write_file_path,
            } => Self::read(read_file_path, false, true, Some(write_file_path)),
        }
    }

    fn read(read_file_path: String, stdout: bool, output: bool, write_file_path: Option<String>) {
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
                    let data: Result<ApplicationPacket, bincode::error::DecodeError> =
                        decode_from_std_read(&mut reader, standard());

                    match data {
                        Ok(packet) => {

                            // Temp
                            let mut wrt = Writer::from_path("/home/supergoodname77/Desktop/Final Flash/AMALTHEA/data-cli/src/temp.csv").expect("Wrong path for csv");
                            //
                            if stdout {
                                println!("{packet:#?}");
                                let value = serde_json::to_value(packet).unwrap();
                                if let Some(headers_map) = Commands::parse_packet(&value) {
                                    
                                    ///
                                    /// Need to name new csv file as the name of the struct.
                                    /// If the name already exists, we can writing the headers and only write the values
                                    /// of the current iteration.
                                    /// Also needs some mechanism for when to start and stop some new iteration of a 
                                    /// csv file for the same struct.
                                    //

                                    // headers_map.iter().map(|k,v| {
                                    if 
                                    headers_map.keys().next()
                                    // })

                                    // let mut headers_keys = headers_map.keys();
                                    // println!("{headers_keys:#?}");
                                    // // headers_map.values();
                                    // wrt.write_record(headers_keys.clone()).expect("Ooops, failure writing headers");
                                    // let mut output_buf = Vec::new();
                                    // // println!("{headers_keys:#?}");

                                    // for key in &mut headers_keys {
                                    //     output_buf.push(&headers_map[key]);
                                    //     // wrt.write_record(headers_keys)key
                                    // }
                                    // // println!("{output_buf:#?}");
                                    // wrt.write_record(output_buf).expect("Ooops, failure writing values");
                                }
                                
                            }
                            if output {
                                if let Some(ref mut file_writer) = writer {
                                    Commands::file_write(packet, file_writer);
                                }
                            }
                        }
                        Err(e) => match e {
                            bincode::error::DecodeError::Io { inner, .. } => {
                                if inner.kind() == ErrorKind::UnexpectedEof {
                                    break;
                                }
                            }
                            _ => eprintln!("Nooo error {e}"),
                        },
                    }
                }
            }

            Err(e) => {
                eprintln!("Error reading raw data from file: {e}")
            }
        }
    }

    fn file_write(packet: ApplicationPacket, file_writer: &mut BufWriter<File>) {
        if let Err(e) = writeln!(file_writer, "{packet:#?}") {
            eprintln!("Error appending to output file: {e}")
        }
    }

    fn parse_packet(mut packet: &serde_json::Value) -> Option<HashMap<String, String>> {

        let mut result = None;

        if let Some(field) = packet.as_object(){

            let mut incomplete_map = HashMap::new();
            for key in field.keys() {
                if let Some(map) = Commands::parse_packet(&field[key]) {
                    // Over here an insertion would have to be added if we also want to view the name
                    // of sub_structs, but this might be unneccessary because it is counter intuitive to the
                    // format of CSV files
                    incomplete_map.extend(map);
                } else {
                    incomplete_map.insert(key.to_string(), field[key].to_string());
                }
            }
            result = Some(incomplete_map);
        }

        result
    }
}
fn main() {
    let cli = Cli::parse();
    cli.command.execute();
}
