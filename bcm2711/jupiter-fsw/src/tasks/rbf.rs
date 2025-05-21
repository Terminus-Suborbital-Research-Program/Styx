use std::sync::{Arc, Mutex};
use std::thread::spawn;

use common::rbf::RbfState;
use common::rbf::{ActiveHighRbf, RbfIndicator};

use crate::gpio::read::ReadPin;

/// The task spawner for the RBF reader
pub struct RbfTask {
    indicator: ActiveHighRbf<ReadPin>,
    state: Arc<Mutex<RbfState>>,
}

impl RbfTask {
    pub fn new(mut indicator: ActiveHighRbf<ReadPin>) -> Self {
        let state = indicator.get_inhibition();
        Self {
            indicator,
            state: Arc::new(Mutex::new(state)),
        }
    }

    pub fn spawn(self, interval_ms: u64) -> RbfReader {
        let update_state = self.state.clone();
        spawn(move || {
            rbf_states_thread(self.indicator, update_state, interval_ms);
        });
        RbfReader::from(self.state)
    }
}

#[derive(Clone)]
pub struct RbfReader {
    rbf: Arc<Mutex<RbfState>>,
}

impl RbfReader {
    pub fn read(&self) -> RbfState {
        *self.rbf.lock().unwrap()
    }
}

impl From<Arc<Mutex<RbfState>>> for RbfReader {
    fn from(rbf: Arc<Mutex<RbfState>>) -> Self {
        Self { rbf }
    }
}

fn rbf_states_thread<T: RbfIndicator>(
    mut indicator: T,
    state: Arc<Mutex<RbfState>>,
    update_interval: u64,
) -> ! {
    loop {
        {
            // Explicit context
            let mut state = state.lock().unwrap();
            *state = indicator.get_inhibition();
        }
        std::thread::sleep(std::time::Duration::from_millis(update_interval));
    }
}

