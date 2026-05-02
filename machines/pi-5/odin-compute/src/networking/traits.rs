use crate::networking::error::IOError;
use bin_packets::packets::ApplicationPacket;
trait NetworkSocket {
    fn send(application_packet: &ApplicationPacket) -> Result<(), IOError>;
}
