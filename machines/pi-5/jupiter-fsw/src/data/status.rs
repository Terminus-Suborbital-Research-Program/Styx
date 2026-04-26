// Status as defined in the Payload Status Indicator Board LED Matrix:
// This struct will not handle all statuses, as ejector is aware of it's own state
// as well as the state of its servos

// Therefore this shall handle:
// Jupiter state
// Guard state
// Infratracker State
// Radio states ~ White while odin in stasis ~
// Odin Compute ~ White while odin in stasis
// Odin pico ~ White while odin in stasis
// Jupiter Avionics ~ 


// Meanwhile Ejector will already be aware of and handle:
// RBF State
// Servo State
// Ejector state
// Ejector pico

use bin_packets::{
    phases::JupiterPhase, 
    device::{Device, PacketWriter},
    rgbstatus::{RGBOptions, WireColor}
};
use std::{io::Write, time::{Instant, Duration}};
const STATUS_TIMEOUT: Duration = Duration::from_secs(10);

pub const COLOR_RED: Option<WireColor>     = Some(WireColor::new(50, 0, 0));
pub const COLOR_GREEN: Option<WireColor>   = Some(WireColor::new(0, 50, 0));
pub const COLOR_BLUE: Option<WireColor>    = Some(WireColor::new(0, 0, 50));

pub const COLOR_CYAN: Option<WireColor>    = Some(WireColor::new(0, 50, 50));
pub const COLOR_YELLOW: Option<WireColor>  = Some(WireColor::new(50, 50, 0));
pub const COLOR_MAGENTA: Option<WireColor> = Some(WireColor::new(50, 0, 50));

pub const COLOR_ORANGE: Option<WireColor>  = Some(WireColor::new(50, 20, 0));
pub const COLOR_PURPLE: Option<WireColor>  = Some(WireColor::new(25, 0, 50));
pub const COLOR_PINK: Option<WireColor>    = Some(WireColor::new(50, 15, 30));

pub const COLOR_OFF: Option<WireColor>     = Some(WireColor::new(0, 0, 0));



pub struct ExperimentColorState {
    latest_geiger: Option<Instant>,
    latest_thermocouple: Option<Instant>,
    latest_infratracker: Option<Instant>,
    latest_avionics: Option<Instant>,
    latest_phase: JupiterPhase,
}   

impl ExperimentColorState {
    pub fn new() -> Self {
        Self {
            latest_geiger: None,
            latest_thermocouple: None,
            latest_infratracker: None,
            latest_avionics: None,
            latest_phase: JupiterPhase::PowerOn,
        }
    }

    pub fn feed_geiger(&mut self) { self.latest_geiger = Some(Instant::now()); }
    pub fn feed_thermocouple(&mut self) { self.latest_thermocouple = Some(Instant::now()); }
    pub fn feed_infratracker(&mut self) { self.latest_infratracker = Some(Instant::now()); }
    pub fn feed_avionics(&mut self) { self.latest_avionics = Some(Instant::now()); }
    
    pub fn feed_jupiter_state_machine(&mut self, jupiter_phase: JupiterPhase) {
        self.latest_phase = jupiter_phase;
    }

    fn is_active(latest: Option<Instant>, now: Instant) -> bool {
        latest.is_some_and(|t| now.duration_since(t) <= STATUS_TIMEOUT)
    }

    pub fn current_status(&self) -> RGBOptions {
        let now = Instant::now();

        let thermo_active = Self::is_active(self.latest_thermocouple, now);
        let geiger_active = Self::is_active(self.latest_geiger, now);
        let infra_active  = Self::is_active(self.latest_infratracker, now);
        let avionics_active = Self::is_active(self.latest_avionics, now);

        let guard_color = match (thermo_active, geiger_active) {
            (true, true)   => COLOR_GREEN,
            (true, false)  => COLOR_YELLOW,
            (false, true)  => COLOR_PINK,
            (false, false) => COLOR_OFF,
        };
        
        RGBOptions { 
            RBF: None, 
            HaLow: None, 
            Esp: None, 
            Infratracker: if infra_active { COLOR_CYAN } else { COLOR_OFF }, 
            Guard: guard_color, 
            Jupiter: match self.latest_phase {
                JupiterPhase::PowerOn => COLOR_GREEN,
                JupiterPhase::Launch => COLOR_GREEN,
                JupiterPhase::CamStart => COLOR_BLUE,
                JupiterPhase::RocketDespin => COLOR_MAGENTA,
                JupiterPhase::Infratracking => COLOR_CYAN,
                JupiterPhase::EjectDeployable => COLOR_MAGENTA,
                JupiterPhase::BatteryPower => COLOR_YELLOW,
                JupiterPhase::Shutdown => COLOR_OFF
            }, 
            ElectroMagnet: None, 
            Servos: None, 
            Jupiter_Avionics_Health: if avionics_active { COLOR_GREEN } else { COLOR_OFF }, 
            Ejector_Health: None, 
            Odin_Compute_Health: None, 
            Odin_Pico_Health: None
        }
    }
}

