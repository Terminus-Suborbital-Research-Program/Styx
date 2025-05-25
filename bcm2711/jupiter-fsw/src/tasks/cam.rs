use std::process::{Command, Child};
use crate::timing::t_time_estimate;

pub struct InfraTracker {
    start_t_time: i32,
}

impl InfraTracker {
    pub fn new() -> Self {
        Self {
            start_t_time: t_time_estimate(),
        }
    }

    pub fn spawn(self, image_amount: i32, ) -> Child {
        let script_path = "../../../../../COTS-Star-Tracker-Amalthea/infratracker.py";
        // Args- delay between camera shots
        Command::new("python")
            .arg(script_path)
            .arg(self.start_t_time.to_string())
            .spawn()
            .expect("Oopsy, the script failed to start")
    }
 
}