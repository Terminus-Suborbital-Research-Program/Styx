
use bin_packets::packets::ApplicationPacket;
use crate::networking::error::IOError;
trait NetworkSocket {
    fn send(application_packet: &ApplicationPacket) -> Result<(),IOError>;
}