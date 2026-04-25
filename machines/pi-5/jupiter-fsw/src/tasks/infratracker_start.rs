use std::{fs::create_dir, sync::{atomic::AtomicBool, mpsc::Receiver}, thread::sleep, time::Duration};
//use aether::reference_frame::{Body, ICRF};
//use aether::attitude::Quaternion;
use lazy_static::lazy_static;

use log::info;


lazy_static! {
    pub static ref TRACKING: AtomicBool = AtomicBool::new(false);
}

const STAR_TRACKER_DIR: &str = "/home/terminus/basler/";
static buffer_time:u64 = 1000;


struct InfratrackerTask {
    reciever: Receiver<Quaternion<f32, ICRF<f32>, Body<f32>>>,
}

impl InfratrackerTask {

    pub fn new(reciever: Receiver<Quaternion<f32, ICRF<f32>, Body<f32>>>) -> Self {
        Self { reciever }
    }
pub fn save_data(&mut self) -> ! {
   info!("Starting Basler camera!");

    create_dir(STAR_TRACKER_DIR).ok();


    loop { 
        if (TRACKING.load(std::sync::atomic::Ordering::Relaxed)) {
            info!("Call The Reciever");
            // Write to file
        }

        sleep(Duration::from_millis(buffer_time));
    }
    
}
}

pub fn spawn_i_thread(mut th: InfratrackerTask) {
    std::thread::spawn(move || th.save_data());
}