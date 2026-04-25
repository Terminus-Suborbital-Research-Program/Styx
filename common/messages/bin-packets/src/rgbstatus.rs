use bincode::{Decode, Encode};
use defmt::Format;
use serde::{Deserialize, Serialize};
use smart_leds::RGB8;

// Have to make seperate type for color because of Rust's Orphan rule
#[derive(Debug, Clone, Copy, Encode, Decode, Format, Deserialize, Serialize)]
pub struct WireColor { 
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl From<WireColor> for RGB8 {
    fn from(wire: WireColor) -> Self {
        RGB8::new(wire.r, wire.g, wire.b)
    }
}

#[derive(Debug, Clone, Copy, Encode, Decode, Format, Deserialize, Serialize)]
pub struct RGBOptions {
    pub RBF: Option<WireColor>,
    pub HaLow: Option<WireColor>,
    pub Esp: Option<WireColor>,
    pub Infratracker: Option<WireColor>,
    pub Guard: Option<WireColor>,
    pub Jupiter: Option<WireColor>,
    pub ElectroMagnet: Option<WireColor>,
    pub Servos: Option<WireColor>,
    pub Jupiter_Avionics_Health: Option<WireColor>,
    pub Ejector_Health: Option<WireColor>,
    pub Odin_Compute_Health: Option<WireColor>,
    pub Odin_Pico_Health: Option<WireColor>,
}
