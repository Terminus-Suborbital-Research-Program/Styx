use bincode::{Encode, config};
use bin_packets::packets::{ApplicationPacket, testing::*};

pub fn send_packet<T: Encode>(packet: T) -> () {
    let y = bincode::encode_to_vec(packet, config::standard()).unwrap();
}
