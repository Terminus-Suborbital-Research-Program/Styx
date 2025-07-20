
use indexmap::IndexMap;
use chrono::prelude::*;
use bin_packets::packets::ApplicationPacket;
use bincode::{config::standard, decode_from_std_read};
use csv::Writer;

use std::{
    collections::{HashMap, HashSet}, 
    fs::{read_dir, File, OpenOptions}, 
    io::{self, BufReader, BufWriter, ErrorKind, Read, Write}, 
    path::{Path, PathBuf}
};

// Manage the process of writing lines to an existing file
// if the packet was already written during the duration of execution
// or writing to a new file if the packet is newly recieved in this run of the CLI,
// and in this case determining whether to write the new file with an increment format
// or a datetime stamp
pub struct CSVPacketTranslator {
    created_file_list: HashSet<String>,
    output_directory: PathBuf,
    current_time: DateTime<Local>,
    file_name_format: FileNameFormat,
}

pub enum FileNameFormat {
    Iterate,
    Timestamp,
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
