use crate::record::packet::SdrPacketLog;
use bincode::{
    config::standard,
    decode_from_slice, encode_into_slice,
    serde::{decode_from_std_read, encode_into_std_write},
};
use std::fs::File;
use std::path::PathBuf;
use std::{
    fs,
    io::{BufReader, BufWriter, Write},
};

pub struct SignalLogger {
    writer: BufWriter<File>,
}

impl SignalLogger {
    pub fn new(file_path: PathBuf) -> Self {
        let file = File::create(file_path).expect("Could not open baseline file!");
        Self {
            writer: BufWriter::new(file),
        }
    }

    pub fn log_packet(&mut self, packet: SdrPacketLog) {
        encode_into_std_write(&packet, &mut self.writer, standard()).unwrap();
    }

    pub fn record_psd(&mut self, power_spectrum_bin_averaged: Vec<f32>) {
        encode_into_std_write(&power_spectrum_bin_averaged, &mut self.writer, standard()).unwrap();
    }
}

pub struct SignalReader {
    reader: BufReader<File>,
}
impl SignalReader {
    pub fn new(file_path: PathBuf) -> Self {
        let file = File::open(file_path).expect("Could not open baseline file!");
        Self {
            reader: BufReader::new(file),
        }
    }

    pub fn read_psd(&mut self) -> Vec<f32> {
        let expected_average =
            decode_from_std_read(&mut self.reader, standard()).expect("Failed to decode PSD data");
        expected_average
    }
}
