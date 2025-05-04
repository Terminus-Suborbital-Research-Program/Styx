use std::{
    sync::{Arc, RwLock},
    time::Instant,
};

use bin_packets::phases::JupiterPhase;

use crate::tasks::PinStates;

pub struct JupiterStateMachine {
    phase: JupiterPhase,
    phase_start_time: Instant,
    pin_struct: Arc<RwLock<PinStates>>,
}

impl JupiterStateMachine {
    pub fn new(pins: Arc<RwLock<PinStates>>) -> Self {
        JupiterStateMachine {
            phase: JupiterPhase::PowerOn,
            phase_start_time: Instant::now(), //Should change this to be uninitalized
            pin_struct: pins,
        }
    }

    pub fn current_phase(&self) -> JupiterPhase {
        self.phase
    }

    pub fn update(&mut self) -> Option<JupiterPhase> {
        let old_phase = self.phase;
        let pins = self.pin_struct.read().unwrap();
        self.phase = match self.current_phase() {
            JupiterPhase::PowerOn => {
                if self.phase_start_time.elapsed().as_secs() > 120 {
                    JupiterPhase::MainCamStart
                } else {
                    JupiterPhase::PowerOn
                }
            }

            JupiterPhase::MainCamStart => {
                if self.phase_start_time.elapsed().as_secs() > 60 {
                    JupiterPhase::Launch
                } else {
                    JupiterPhase::MainCamStart
                }
            }

            JupiterPhase::Launch => {
                if pins.te_1_high() {
                    JupiterPhase::SkirtEjection
                } else if self.phase_start_time.elapsed().as_secs() > 70 {
                    JupiterPhase::SecondaryCamStart
                } else {
                    JupiterPhase::Launch
                }
            }

            JupiterPhase::SecondaryCamStart => {
                if self.phase_start_time.elapsed().as_secs() > 40 || pins.te_1_high() {
                    JupiterPhase::SkirtEjection
                } else {
                    JupiterPhase::SecondaryCamStart
                }
            }

            JupiterPhase::SkirtEjection => {
                if pins.te_2_high() {
                    JupiterPhase::BatteryPower
                } else {
                    JupiterPhase::SkirtEjection
                }
            }

            JupiterPhase::BatteryPower => {
                if self.phase_start_time.elapsed().as_secs() > 60 {
                    JupiterPhase::Shutdown
                } else {
                    JupiterPhase::BatteryPower
                }
            }

            JupiterPhase::Shutdown => JupiterPhase::Shutdown,
        };

        if old_phase == self.phase {
            None
        } else {
            self.phase_start_time = Instant::now();
            Some(self.current_phase())
        }
    }
}
