use aether::terrestrial::wgs84::constants::G;
use bin_packets::{phases::JupiterPhase, rgbstatus::{
    RGBOptions, WireColor
}};

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

const STATUS_TIMEOUT: u64 = 10;
use std::{io::Write, time::Instant};
use bin_packets::device::{Device, PacketWriter};

pub enum GuardStates {
    All,
    ThermoOnly,
    ScintillatorOnly,
    None,
}

pub struct ExperimentColorState {
    latest_guard: u64,
    latest_infratracker: u64,
    latest_avionics: u64,

    latest_guard_state: GuardStates,
    latest_phase: JupiterPhase,

    guard: bool,
    infratracker: bool,
    avionics: bool,
}   

impl ExperimentColorState {

    pub fn new() -> Self {
        Self {
            latest_guard: 0,
            latest_infratracker: 0,
            latest_avionics: 0,

            latest_guard_state: GuardStates::None,
            latest_phase: JupiterPhase::PowerOn,

            guard: false,
            infratracker: false,
            avionics: false,


        }
    }
    pub fn feed_guard(&mut self, guard_state: GuardStates) {
        self.latest_guard = Instant::now();
        self.latest_guard_state = guard_state;
    }

    pub fn feed_infratracker(&mut self) {
        self.latest_infratracker = Instant::now();
    }

    pub fn feed_avionics(&mut self) {
        self.latest_infratracker = Instant::now();
    }

    // Doesn't actually need to feed but naming convention is the
    // same so the call isn't forgotten
    pub fn feed_jupiter_state_machine(&mut self, jupiter_phase: JupiterPhase) {
        self.latest_phase = jupiter_phase;
    }


    pub fn current_status(&mut self) -> RGBOptions {
        let now = Instant::now();

        let guard_time = now.duration_since(self.latest_guard);
        let infratracker_time = now.duration_since(self.latest_infratracker);
        let avionics_time = now.duration_since(self.latest_avionics);

        update_experiment_flag(&guard_time, self.guard);
        update_experiment_flag(&infratracker_time, self.infratracker);
        update_experiment_flag(&avionics_time, self.avionics);

        
        RGBOptions { 
            RBF: None, 
            HaLow: None, 
            Esp: None, 
            Infratracker: if self.infratracker {COLOR_CYAN } else {COLOR_OFF}, 
            Guard: if self.guard {
                match self.latest_guard_state {
                    GuardStates::All => COLOR_GREEN,
                    GuardStates::ThermoOnly => COLOR_YELLOW,
                    GuardStates::ScintillatorOnly => COLOR_PINK,
                    GuardStates::None => COLOR_OFF,
                }
            } else {
                COLOR_OFF
            }, 
            Jupiter: 
                match self.latest_phase {
                    JupiterPhase::PowerOn => COLOR_GREEN,
                    JupiterPhase::Launch => COLOR_GREEN,
                    JupiterPhase::CamStart => COLOR_BLUE,
                    JupiterPhase::RocketDespin => COLOR_MAGENTA,
                    JupiterPhase::Infratracking => COLOR_CYAN,
                    JupiterPhase::EjectDeployable => COLOR_MAGENTA,
                    JupiterPhase::BatteryPower => COLOR_YELLOW,
                    JupiterPhase::Shutdown => COLOR_OFF
            } , 
            ElectroMagnet: None, 
            Servos: None, 
            Jupiter_Avionics_Health: if self.avionics { COLOR_GREEN } else { COLOR_OFF}, 
            Ejector_Health: None, 
            Odin_Compute_Health: None, 
            Odin_Pico_Health: None
        }



    }

    pub fn update_experiment_flag(latest_time: &Duration, current_state: &mut bool) {
        if latest_time > STATUS_TIMEOUT { 
            current_state = false; 
        } else {
            current_state = !current_state;
        }
    }


}

