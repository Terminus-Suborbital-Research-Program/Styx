use bin_packets::packets::ApplicationPacket;
use bincode::{config::standard, decode_from_std_read};
use clap::{Parser, Subcommand};
use csv::Writer;
use indexmap::IndexMap;
use chrono::prelude::*;

use std::{
    collections::{HashMap, HashSet}, 
    fs::{read_dir, File, OpenOptions}, 
    io::{self, BufReader, BufWriter, ErrorKind, Read, Write}, 
    path::{Path, PathBuf}
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
        #[arg(help = "Path for the directory to save readable data (folder, not file)")]
        write_file_path: Option<String>,
    },

    Write {
        #[arg(help = "Raw binary file to convert")]
        read_file_path: String,
        #[arg(help = "Path for the directory to save readable data (folder, not file)")]
        write_file_path: String,
    },
}

struct CSVPacketTranslator {
    file_list: HashSet<String>,
    output_directory: PathBuf,
    current_time: DateTime<Local>,
}


impl CSVPacketTranslator {
    pub fn new(output_path: &Path) -> Result<Self, std::io::Error> {
        let path_iter = read_dir(output_path).expect("Failure to create directory iterator");

        // Collect file names of files in provided directory to check against later
        let file_list: Result<HashSet<String>,std::io::Error> =
        path_iter.map(|file_query| {
            match file_query {
                Ok(file_entry) => {
                    match file_entry.file_name().into_string() {
                        Ok(file_string) => Ok(file_string),

                        Err(os_string) => {
                            Err(io::Error::other("Error converting filename to program readable string"))
                        }
                    }
                }

                Err(e) => {
                    eprintln!("Error reading directory entr: {}",e);
                    Err(e)
                }
            }
        }).collect();

        let mut csv_dir = PathBuf::new();
        csv_dir.push(output_path);


        match file_list {
            Ok(list) => {
                Ok(CSVPacketTranslator {
                    output_directory: csv_dir,
                    file_list: list,
                    current_time: Local::now()
                })

            }
            Err(e) => Err(e)
        }
    }

    

    fn parse_packet(&self, packet: &serde_json::Value) -> Option<IndexMap<String, String>> {

        let mut result = None;

        if let Some(field) = packet.as_object(){

            let mut incomplete_map = IndexMap::new();
            for key in field.keys() {
                if let Some(map) = self.parse_packet(&field[key]) {
                    // Over here an insertion would have to be added if we also want to view the name
                    // of sub_structs, but this might be unneccessary because it is counter intuitive to the
                    // format of CSV files
                    incomplete_map.extend(map);
                } else {
                    // Add in primitive values with any quotes removed
                    incomplete_map.insert(key.to_string(), field[key].to_string().replace(&['(', ')', ',', '\"', ';', ':', '\''][..], ""));
                }
            }
            result = Some(incomplete_map);
        }

        result
    }

    pub fn file_write(&mut self, packet: ApplicationPacket) {
        // Turn the packet into a serde JSON value, from which we can recieve 
        // - the struct name (for determining if a csv file already exists for this struct)
        // - a map of the keys (struct field names) and values (primitives) contained within the struct
        let packet_struct = serde_json::to_value(packet).unwrap();

        // Get a map of the serde JSON value
        if let Some(field) = packet_struct.as_object(){

            // Get the title of the serde Json value
            match field.keys().next() {
                // If we get a name for the struct, fine if we have a matching csv filename already in the 
                // provided directory
                Some(struct_name) => {
                    // Create the writer to write to the csv file for this specific struct

                    // Find if csv files have previously been created in this directory
                    // if so, increment a new csv file for the packet
                    // let file_iteration = self.file_list
                    //                                     .iter()
                    //                                     .filter(|file| file.starts_with(struct_name))
                    //                                     .count() + 1;

                    

                    // Inefficient dogshit, rework this later
                    // let time = &self.current_time.format("%m/%d/%Y %H:%M").to_string();
                    let time = &self.current_time.format("%m-%d-%Y %H:%M:%S").to_string();
;
                    let mut file_path = self.output_directory.clone().into_os_string();
                    // println!("{formatted_time}");
                    let file_name = format!("{struct_name} - {time} .csv");
                    file_path.push(&file_name);

                    // Open file and csv writer in append mode
                    let output_file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(file_path)
                        .expect("Uh oh, output file couldn't open");
                    let mut writer = Writer::from_writer(output_file);
                    
                    // Get the map of all struct values and append the values in csv format to the file
                    if let Some(headers_map) = self.parse_packet(&packet_struct) {

                        
                        // match file_iteration {
                        //     1 => 
                        // }

                        if self.file_list.contains(&file_name) {
                            writer.write_record(headers_map.values()).unwrap();
                            
                        } else {
                            // Create new file with headers, and list 
                            writer.write_record(headers_map.keys()).unwrap();
                            writer.write_record(headers_map.values()).unwrap();
                            self.file_list.insert(file_name);
                        }
                    }
                    // If the struct name is known in our internal list, we can safely assume we have an old file
                    // and just append the values to the existing csv without adding the headers
                }

                None => {
                    eprintln!("Error parsing struct name of recieved packet");
                }
            }
        }

    }

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
        // Open file for reading and create file reader
        let file = File::open(&read_file_path);

        match file {
            Ok(file) => {
                let mut reader = BufReader::new(file);

                let mut writer: Option<CSVPacketTranslator> = None;
                // Prepare CSV writer if we are outputting
                if output {
                    if let Some(output_path) = write_file_path {
                        writer = Some(CSVPacketTranslator::new(Path::new(&output_path))
                                       .expect("Error Creating Packet Translator: "));
                    }
                }
                loop {
                    // decode packet
                    let data: Result<ApplicationPacket, bincode::error::DecodeError> =
                        decode_from_std_read(&mut reader, standard());

                    match data {
                        Ok(packet) => {
                            // Print to console in read mode
                            if stdout {
                                println!("{packet:#?}");
                            }
                            // Write csv files to directory in read mode with -o flag, or in write mode
                            if output {
                                if let Some(ref mut file_writer) = writer {
                                    file_writer.file_write(packet);
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

    
    
}
fn main() {
    let cli = Cli::parse();
    cli.command.execute();
}
