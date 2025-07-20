
use crate::csv_translator::CSVPacketTranslator;

use bin_packets::packets::ApplicationPacket;
use bincode::{config::standard, decode_from_std_read};

use std::{
    fs::File,
    io::{BufReader, ErrorKind}, 
    path::Path
};

// Responsible to determine where we're writing to (currently either stdout or another file)
// and
pub struct DataParser {
    pub write_to_stdout: bool,
    pub csv_packet_translator: Option<CSVPacketTranslator>,
}

impl DataParser {
    pub fn parse_file(mut self, read_file_path: &Path) {

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
