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
        #[arg(long, short, help = "Date and Timestamp the File")]
        time: bool,
        #[arg(long, short, help = "Iterate the filename or overwrite?")]
        iterate: bool,
    },
}


// Manage the process of writing lines to an existing file
// if the packet was already written during the duration of execution
// or writing to a new file if the packet is newly recieved in this run of the CLI,
// and in this case determining whether to write the new file with an increment format
// or a datetime stamp
struct CSVPacketTranslator {
    created_file_list: HashSet<String>,
    output_directory: PathBuf,
    current_time: DateTime<Local>,
    file_name_format: FileNameFormat,
}

enum FileNameFormat {
    Iterate,
    Timestamp,
}

// Responsible to determine where we're writing to (currently either stdout or another file)
// and
struct DataParser {
    write_to_stdout: bool,
    csv_packet_translator: Option<CSVPacketTranslator>,
}

impl DataParser {
    fn parse_file(mut self, read_file_path: &Path) {

        let file = File::open(&read_file_path);

        match file {
            Ok(file) => {
                let mut reader = BufReader::new(file);

                loop {
                    // decode packet
                    let data: Result<ApplicationPacket, bincode::error::DecodeError> =
                        decode_from_std_read(&mut reader, standard());

                    match data {
                        Ok(packet) => {
                           self.write_decoded_packet(packet);
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

    fn write_decoded_packet(&mut self, packet: ApplicationPacket) {
        // If we flagged to write to console, do so
        if self.write_to_stdout {
            println!("{packet:#?}");
        }
        // If we flagged to write to file, and provided a path, the csv packet translator should be available
        // and we can to do so
        if let Some(csv_translator) = &mut self.csv_packet_translator {
            csv_translator.file_write(packet)
        }
    }
}

// A builder for the data parser following the builder design pattern https://refactoring.guru/design-patterns/builder/rust/example
//
// Done because the initilization logic was getting complex, and I want the data parser struct to be extensible
// and easily configurable for potential future use with a serial listening daemon
struct DataParserBuilder {
    write_to_file: bool,
    output_file_path: Option<PathBuf>,
    write_to_stdout: bool,
    file_name_format: FileNameFormat,
}

impl DataParserBuilder {

    // In the future this could have read file and listen_to_serial

    fn new() -> DataParserBuilder {
        DataParserBuilder {
            write_to_file: false,
            output_file_path: None,
            write_to_stdout: false,
            // For now leaving this as a default option, but eventually might turn this to a None option variant
            // if the dataparser grows and I want to just have a default config
            file_name_format: FileNameFormat::Iterate,

        }
    }


    fn write_to_stdout(mut self, write_to_stdout: bool) -> Self {
        self.write_to_stdout = write_to_stdout;
        self
    }

    fn write_to_file(mut self, write_to_file: bool, output_path: PathBuf) -> Self  {
        self.write_to_file = write_to_file;
        self.output_file_path = Some(output_path);
        self
    }

    fn time(mut self, time: bool) -> Self {
        if time {
            self.file_name_format = FileNameFormat::Timestamp;
        }
        self
    }

    fn iterate(mut self, iterate: bool) -> Self {
        if iterate {
            self.file_name_format = FileNameFormat::Iterate;
        }
        self
    }

    fn build<'a>(self) -> DataParser {

        let mut csv_packet_translator: Option<CSVPacketTranslator> = None;

        if self.write_to_file {
            match self.output_file_path {
                Some(path) => {
                    csv_packet_translator = Some(CSVPacketTranslator::new(path, self.file_name_format)
                                            .expect("Error creating csv translator: "))
                }

                None => {
                    panic!("No path provided on call")
                }
            }
        }
        // Panic case should not be neccessary because the default of each command should
        // include at least one method of output, be it console or file

        DataParser { 
            write_to_stdout: self.write_to_stdout,
            csv_packet_translator: csv_packet_translator,
        }
    }
}


// Handles the logic behind dynamically generating csv headers, 
// lines, and different files based on newly received packet types
impl CSVPacketTranslator {
    pub fn new(output_path: PathBuf, file_name_format: FileNameFormat) -> Result<Self, std::io::Error> {
        let path_iter = read_dir(&output_path).expect("Failure to create directory iterator");


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

        match file_list {
            Ok(list) => {
                Ok(CSVPacketTranslator {
                    output_directory: output_path,
                    created_file_list: list,
                    current_time: Local::now(),
                    file_name_format: file_name_format,
                })

            }
            Err(e) => Err(e)
        }
    }

    
    // Recursively collect the headers of a struct
    // This is recursive because some structs can have many layers of substructs
    // with their own headers that must also be collected
    fn collect_packet_headers(&self, packet: &serde_json::Value) -> Option<IndexMap<String, String>> {

        let mut result = None;

        if let Some(field) = packet.as_object(){

            let mut incomplete_map = IndexMap::new();
            for key in field.keys() {
                if let Some(map) = self.collect_packet_headers(&field[key]) {
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
                    let file_iteration = self.created_file_list
                                                        .iter()
                                                        .filter(|file| file.starts_with(struct_name))
                                                        .count() + 1;

                    // Inefficient dogshit, rework this later
                    // let time = &self.current_time.format("%m/%d/%Y %H:%M").to_string();
                    let time = &self.current_time.format("%m-%d-%Y %H:%M:%S").to_string();

                    let mut file_path = self.output_directory.clone().into_os_string();

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
                    if let Some(headers_map) = self.collect_packet_headers(&packet_struct) {

                        if self.created_file_list.contains(&file_name) {
                            writer.write_record(headers_map.values()).unwrap();
                        } else {
                            // Create new file with headers, and list 
                            writer.write_record(headers_map.keys()).unwrap();
                            writer.write_record(headers_map.values()).unwrap();
                            
                            self.created_file_list.insert(file_name);
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
            } => {
                let data_parser = 
                DataParserBuilder::new()
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
                iterate
            } => {
                let data_parser = 
                DataParserBuilder::new()
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
