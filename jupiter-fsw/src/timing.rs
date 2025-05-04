use lazy_static::lazy_static;
use std::time::SystemTime;

static POWER_ON_T_ESTIMATE_SEC: i32 = -120;

// Lazy static for the time we turned on using a system time
lazy_static! {
    pub static ref POWER_ON_TIME: SystemTime = SystemTime::now();
}

// Gets the time since power on in seconds
pub fn power_on_time() -> i32 {
    let now = SystemTime::now();
    let duration = now.duration_since(*POWER_ON_TIME).unwrap();
    duration.as_secs() as i32
}

// Gets our T-time estimate using the time since power on and the boot time estimate
pub fn t_time_estimate() -> i32 {
    power_on_time() + POWER_ON_T_ESTIMATE_SEC
}
