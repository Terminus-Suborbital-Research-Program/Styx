use bincode::{Decode, Encode};
use ws2812_rs::Color;
use defmt::Format;

use serde::{Deserialize, Serialize};


// Have to make seperate type for color because of Rust's Orphan rule
#[derive(Debug, Clone, Copy, Encode, Decode, Format, Deserialize, Serialize )]
pub struct WireColor(pub [u8; 3]);

impl From<WireColor> for Color {
    fn from(wire: WireColor) -> Self {
        Color(wire.0)
    }
}

#[derive(Debug, Clone, Copy, Encode, Decode, Format, Deserialize, Serialize )]
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