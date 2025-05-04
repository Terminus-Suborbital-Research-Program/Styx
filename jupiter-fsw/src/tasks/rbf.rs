use rppal::gpio::InputPin;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use log::warn;

#[allow(dead_code)]
pub fn rbf_monitor_thread(rbf_pin: InputPin, rbf_status: Arc<RwLock<bool>>) -> ! {
    // Only grab the lock if this is a new change in state
    let mut previous_status = false;

    loop {
        if transitioned_on(&rbf_pin, &previous_status) {
            match rbf_status.write() {
                Ok(mut status_writer) => {
                    *status_writer = true;
                }
                Err(e) => {
                    warn!("Error getting writer! Error: {:?}", e);
                }
            }
            previous_status = true
        } else if transitioned_off(&rbf_pin, &previous_status) {
            match rbf_status.write() {
                Ok(mut status_writer) => {
                    *status_writer = false;
                }
                Err(e) => {
                    warn!("Error getting writer! Error: {:?}", e);
                }
            }
            previous_status = false
        }
        std::thread::sleep(Duration::from_millis(1000));
    }
}

fn transitioned_on(rbf_pin: &InputPin, previous_status: &bool) -> bool {
    rbf_pin.is_high() && !previous_status
}

fn transitioned_off(rbf_pin: &InputPin, previous_status: &bool) -> bool {
    rbf_pin.is_low() && *previous_status
}
