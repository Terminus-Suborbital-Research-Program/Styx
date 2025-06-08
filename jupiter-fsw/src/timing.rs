use lazy_static::lazy_static;
use log::info;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, SystemTime};

/// The original “guess” we started with at power-on.
static POWER_ON_T_ESTIMATE_SEC: i32 = -120;

lazy_static! {
    pub static ref POWER_ON_TIME: SystemTime = SystemTime::now();
}

lazy_static! {
    static ref T_CALIBRATION_OFFSET: AtomicI32 = AtomicI32::new(POWER_ON_T_ESTIMATE_SEC);
}

/// Seconds elapsed since power-on.
pub fn power_on_time() -> i32 {
    let now = SystemTime::now();
    let dur = now
        .duration_since(*POWER_ON_TIME)
        .unwrap_or(Duration::from_secs(0));
    dur.as_secs() as i32
}

pub fn t_time_estimate() -> i32 {
    power_on_time() + T_CALIBRATION_OFFSET.load(Ordering::Relaxed)
}

pub fn calibrate_to(truth: i32) {
    let elapsed = power_on_time();
    // we want: elapsed + new_offset == truth  →  new_offset = truth - elapsed
    T_CALIBRATION_OFFSET.store(truth - elapsed, Ordering::Relaxed);
    info!("Calibrated time to {truth}");
}
