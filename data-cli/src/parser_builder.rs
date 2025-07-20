
use crate::parser::DataParser;
use crate::csv_translator::{CSVPacketTranslator, FileNameFormat};
use std::path::PathBuf;

// Manage the process of w
// A builder for the data parser following the builder design pattern https://refactoring.guru/design-patterns/builder/rust/example
//
// Done because the initilization logic was getting complex, and I want the data parser struct to be extensible
// and easily configurable for potential future use with a serial listening daemon
pub struct DataParserBuilder {
    write_to_file: bool,
    output_file_path: Option<PathBuf>,
    write_to_stdout: bool,
    file_name_format: FileNameFormat,
}

impl DataParserBuilder {

    // In the future this could have read file and listen_to_serial

    pub fn new() -> DataParserBuilder {
        DataParserBuilder {
            write_to_file: false,
            output_file_path: None,
            write_to_stdout: false,
            // For now leaving this as a default option, but eventually might turn this to a None option variant
            // if the dataparser grows and I want to just have a default config
            file_name_format: FileNameFormat::Iterate,

        }
    }


    pub fn write_to_stdout(mut self, write_to_stdout: bool) -> Self {
        self.write_to_stdout = write_to_stdout;
        self
    }

    pub fn write_to_file(mut self, write_to_file: bool, output_path: PathBuf) -> Self  {
        self.write_to_file = write_to_file;
        self.output_file_path = Some(output_path);
        self
    }

    pub fn time(mut self, time: bool) -> Self {
        if time {
            self.file_name_format = FileNameFormat::Timestamp;
        }
        self
    }

    pub fn iterate(mut self, iterate: bool) -> Self {
        if iterate {
            self.file_name_format = FileNameFormat::Iterate;
        }
        self
    }

    pub fn build<'a>(self) -> DataParser {

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
