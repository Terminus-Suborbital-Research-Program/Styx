use bin_packets::phases::EjectorPhase;
use defmt::info;

/// State machine for the Ejector, holds the current phase
pub struct EjectorStateMachine {
    phase: EjectorPhase,
    next_phase: Option<EjectorPhase>,
}

impl Default for EjectorStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl EjectorStateMachine {
    /// Ejector always enters the Standby phase first
    pub fn new() -> Self {
        Self {
            phase: EjectorPhase::Standby,
            next_phase: None,
        }
    }

    /// Mabye transition to the next phase, depending on conditions
    /// returns the number of ms we should wait before trying to transition again
    pub fn transition(&mut self) -> u64 {
        if let Some(next_phase) = self.next_phase {
            self.phase = next_phase;
            self.next_phase = None;
        }

        let (phase, time) = match self.phase {
            // Standby only moves into ejection if explicitly commanded, so
            // we don't model that here
            EjectorPhase::Standby => (None, 0),

            EjectorPhase::Ejection => (Some(EjectorPhase::Hold), 5000),

            EjectorPhase::Hold => (None, 10000),
        };
        self.next_phase = phase;
        time
    }

    /// Copies the current phase
    pub fn phase(&self) -> EjectorPhase {
        self.phase
    }

    /// Sets the phase to a specific value
    pub fn set_phase(&mut self, phase: EjectorPhase) {
        info!("State Machine: Overriding phase to {}", phase);
        self.phase = phase;
    }
}
