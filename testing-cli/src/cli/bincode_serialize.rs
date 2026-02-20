use bin_packets::packets::{ApplicationPacket, testing::*};
use bincode::{Encode, config};

use crate::cli::serial;

pub fn send_packet<T: Encode>(port_path: &str, baud_rate: u32, packet: T) -> () {
    let packet_vector = bincode::encode_to_vec(packet, config::standard()).unwrap();
    serial::USB::write_serail(port_path, baud_rate, packet_vector);
}
