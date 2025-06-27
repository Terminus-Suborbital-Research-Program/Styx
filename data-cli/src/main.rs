use bin_packets::packets::ApplicationPacket;
use bincode::{config::standard, decode_from_std_read};
use clap::{Parser, Subcommand};
use csv::Writer;
use indexmap::IndexMap;

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

struct CSVPacketTranslator {
    // writer_list: Vec<Writer<File>>,
    file_list: HashSet<String>,
    output_directory: PathBuf
     // Later on this should be done by querying instead of reading all file names every time
}


impl CSVPacketTranslator {
    pub fn new(output_path: &Path) -> Result<Self, std::io::Error> {
        // This shouldn't be required unlist a specific file name is requested, but handling that
        // extraneous case can be done later

        // let csv_directory = output_path.parent().expect("Oops, listed directory does not exist");
        let path_iter = read_dir(output_path).expect("Failure to create directory iterator");

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
                // Remove
                // for file_name in list.clone() {
                //     println!("File name read:{file_name}")
                // }
                //
                Ok(CSVPacketTranslator {
                    output_directory: csv_dir,
                    file_list: list,
                })

            }
            Err(e) => Err(e)
        }
    }


    fn parse_packet(&self, packet: &serde_json::Value) -> Option<HashMap<String, String>> {

        let mut result = None;

        if let Some(field) = packet.as_object(){

            let mut incomplete_map = HashMap::new();
            for key in field.keys() {
                if let Some(map) = self.parse_packet(&field[key]) {
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
                    // self.output_directory.push(format!("/{struct_name}.csv"));

                    // Inefficient dogshit, rework this later
                    let mut file_path = self.output_directory.clone().into_os_string();
                    let file_name = format!("{struct_name}.csv");
                    file_path.push(&file_name);
                    // let b = file_path.to_str().unwrap();
                    // let c = String::from(b);
                    // println!("{}",c);
                    //

                    // Remove
                    println!("{}",file_path.clone().into_string().unwrap());
                    //

                    let output_file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(file_path)
                        .expect("Uh oh, output file couldn't open");

                    let mut writer = Writer::from_writer(output_file);
                    // Check if the path buf is mutated or not later
                    
                    // Get the map of all struct values and append the values in csv format to the file
                    if let Some(headers_map) = self.parse_packet(&packet_struct) {
                        if self.file_list.contains(&file_name) {
                            println!("Old File");
                            writer.write_record(headers_map.values()).unwrap();
                        } else {
                            println!("New File");

                            // This may be detrimental if values are consumed, so cloning on keys may be neccessary,
                            // also this requires index map to order properly
                            writer.write_record(headers_map.keys()).unwrap();
                            writer.write_record(headers_map.values()).unwrap();

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
        let file = File::open(&read_file_path);

        match file {
            Ok(file) => {
                let mut reader = BufReader::new(file);

                let mut writer: Option<CSVPacketTranslator> = None;

                if output {
                    if let Some(output_path) = write_file_path {
                        // let output_file = OpenOptions::new()
                        //     .create(true)
                        //     .append(true)
                        //     .open(output_path)
                        //     .expect("Uh oh, output file couldn't open");
                        // writer = Some(BufWriter::new(output_file));
                        writer = Some(CSVPacketTranslator::new(Path::new(&output_path))
                                       .expect("Error Creating Packet Translator: "));
                    }
                }
                loop {
                    let data: Result<ApplicationPacket, bincode::error::DecodeError> =
                        decode_from_std_read(&mut reader, standard());

                    match data {
                        Ok(packet) => {

                            if stdout {
                                println!("{packet:#?}");
                            }
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

// #[cfg(test)]
// mod tests {
//     // use super::*;

//     use std::path::Path;

//     use crate::CSVPacketTranslator;

//     #[test]
//     fn it_works() {
//         CSVPacketTranslator::new(Path::new("/home/supergoodname77/Desktop/Final Flash/AMALTHEA/data-cli/temp"));

        
//     }
// }
