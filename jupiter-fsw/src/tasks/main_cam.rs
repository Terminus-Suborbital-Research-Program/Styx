use std::{
    fs::{create_dir, read_dir},
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    str::FromStr,
    thread::sleep,
    time::Duration,
};

use log::{error, info};

use crate::timing::t_time_estimate;

const VIDEO_DIRECTORY: &str = "/home/terminus/video/";

/// Internal task for running the actual camera task information
fn camera_task() -> ! {
    // Wait until after the delay
    while t_time_estimate() < -30 {
        sleep(Duration::from_millis(1000));
    }
    info!("Starting main camera!");

    create_dir(VIDEO_DIRECTORY).ok();

    // T+ 302 stop for TE-2
    run_recording_segment(Some(302));

    // T+570 stop
    run_recording_segment(Some(570));

    loop {
        run_recording_segment(None);
    }
}

fn run_recording_segment(stop_at_t: Option<i32>) {
    let mut fail_count = 0;

    'retry: loop {
        // figure out next filename
        let highest = read_dir(VIDEO_DIRECTORY)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                // strip ".avi" and parse
                e.path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.parse::<u32>().ok())
            })
            .max()
            .unwrap_or(0);
        let next = format!("{}.avi", highest + 1);
        let mut file_path = PathBuf::from_str(VIDEO_DIRECTORY).unwrap();
        file_path.push(next);

        // spawn ffmpeg with a piped stdin
        let mut child = Command::new("ffmpeg")
            .args([
                "-f",
                "v4l2",
                "-input_format",
                "mjpeg",
                "-framerate",
                "30",
                "-video_size",
                "1920x1080",
                "-i",
                "/dev/video0",
                "-c:v",
                "copy",
                "-hide_banner",
                "-loglevel",
                "error",
            ])
            .arg(&file_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn ffmpeg");

        // if we should auto-stop, spin up a little watcher
        if let Some(stop_time) = stop_at_t {
            if let Some(mut stdin) = child.stdin.take() {
                std::thread::spawn(move || {
                    // wait until we hit your 570 s mark
                    while t_time_estimate() < stop_time {
                        sleep(Duration::from_millis(200));
                    }
                    // send "q" so ffmpeg writes its trailer and exits cleanly
                    let _ = stdin.write_all(b"q");
                    info!("Sent 'q' to ffmpeg at t={stop_time:.1}");
                });
            }
        }

        // now block until ffmpeg exits
        match child.wait() {
            Ok(status) if status.success() => {
                info!("Finished segment {}", highest + 1);
                break 'retry;
            }
            Ok(status) => {
                error!("ffmpeg exited with {status} – retrying… (fail_count={fail_count})",);
            }
            Err(e) => {
                error!("Failed to wait on ffmpeg: {e} – retrying…");
            }
        }

        // exponential back-off on repeated failures
        fail_count += 1;
        let backoff = if fail_count < 10 { 1_000 } else { 10_000 };
        sleep(Duration::from_millis(backoff));
    }
}

/// Spawn the camera task
pub fn spawn_camera_thread() {
    std::thread::spawn(|| camera_task());
}
