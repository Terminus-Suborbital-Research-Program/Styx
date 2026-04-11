use std::{fs::create_dir, thread::sleep, time::Duration};

use log::info;

const STAR_TRACKER_DIR: &str = "/home/terminus/basler/";

fn basler_cam_task() -> ! {
   info!("Starting Basler camera!");

    create_dir(STAR_TRACKER_DIR).ok();
    loop { 
    
        // Call basler Cam

        // Send Data to Startracker

        // Write to file

        sleep(Duration::from_millis(1000));
    }
    
}