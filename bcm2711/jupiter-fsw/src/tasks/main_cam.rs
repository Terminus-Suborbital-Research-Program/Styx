use std::{
    fs::{create_dir, read_dir},
    path::Path,
    str::FromStr,
    thread::sleep,
    time::Duration,
};

use log::{error, info};

use crate::timing::t_time_estimate;

/// Internal task for running the actual camera task information
fn camera_task() -> ! {
    // Wait until after the delay
    while t_time_estimate() < -30 {
        sleep(Duration::from_millis(1000));
    }
    info!("Starting main camera!");

    let mut video_directory =
        Path::new(&std::env::var("HOME").expect("Who the heck doesn't set a home variable?"))
            .to_path_buf();
    video_directory.push("video");
    // Create, if it doesn't exist
    create_dir(&video_directory).ok();

    loop {
        // Highest integer in directory
        let highest = read_dir(&video_directory)
            .unwrap()
            .filter_map(|x| x.ok())
            .map(|x| String::from_str(x.path().to_str().unwrap_or("0")).unwrap())
            .filter_map(|x| x.parse::<u32>().ok())
            .max()
            .unwrap_or(0);

        let next = format!("{}.avi", highest + 1);

        let mut cmd = std::process::Command::new("ffmpeg")
            .args(["-f", "v4l2"])
            .args(["-input_format", "mjpeg"])
            .args(["-framerate", "30"])
            .args(["-video_size", "1920x1080"])
            .args(["-i", "/dev/video0"])
            .args(["-c:v", "copy"])
            .arg(&next)
            .spawn()
            .unwrap();

        // Run until completion, and then restart
        cmd.wait().ok();
        error!("Camera thread ended unexpectedly!");
    }
}

/// Spawn the camera task
pub fn spawn_camera_thread() {
    std::thread::spawn(|| camera_task());
}
