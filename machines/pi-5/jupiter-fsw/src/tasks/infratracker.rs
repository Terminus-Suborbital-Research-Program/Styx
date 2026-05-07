use std::fs::create_dir;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, sync_channel, Receiver, Sender, TrySendError, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, Instant, UNIX_EPOCH};

use bin_packets::packets::ApplicationPacket;
use lazy_static::lazy_static;
use log::{info, error};

use image::{ImageBuffer, Luma};

use wayfarer::perception::centroiding::Starfinder;
use wayfarer::perception::camera_model::CameraModel;
use wayfarer::startrack::solver::Startracker;
use wayfarer::startrack::quest::quest_real;

use aether::attitude::Quaternion;
use aether::reference_frame::{ICRF, Body};

use DarkAverager::ImageAveragerFromBuffer;

const STAR_TRACKER_DIR: &str = "/home/terminus/basler/";

// capture image and solve every 1Hz or 1000 millis
// Save will happen no matter what but solve can be delayed
const CAPTURE_RATE: u64 = 17;

lazy_static! {
    pub static ref TRACKING: AtomicBool = AtomicBool::new(false);
}

pub struct InfratrackerThread {
    quaternion_sender: Sender<ApplicationPacket>,
}

impl InfratrackerThread {
    pub fn new() -> (Self, Receiver<ApplicationPacket>) {
        let (quaternion_tx, quaternion_rx) = channel();
        (Self { quaternion_sender: quaternion_tx }, quaternion_rx)
    }

    pub fn begin_startracking(self) -> JoinHandle<()> {
        info!("Starting Basler camera!");
        create_dir(STAR_TRACKER_DIR).ok();
        
        thread::spawn(move || {
            let result: Result<(), Box<dyn std::error::Error>> = (|| {
                
                
                let (solver_tx, solver_rx) = sync_channel::<(u64, ImageBuffer<Luma<u8>, Vec<u8>>)>(1);
                let (result_tx, result_rx) = channel::<(u64, Option<Quaternion<f32, ICRF<f32>, Body<f32>>>)>();

                thread::spawn(move || {
                    let starfinder = Starfinder::default();
                    let camera_model = CameraModel::default();
                    let startracker = Startracker::default();

                    // Simple looping thread to that will return solves for every
                    // image recieved until parent dies
                    while let Ok((timestamp, img)) = solver_rx.recv() {
                        let q = Self::solve_attitude(&img, &starfinder, &camera_model, &startracker);
                        let _ = result_tx.send((timestamp, q));
                    }
                });

                
                let pylon = pylon_cxx::Pylon::new();
                let camera = pylon_cxx::TlFactory::instance(&pylon).create_first_device()?;

                let mut pixel_format_node = camera.node_map()?.enum_node("PixelFormat")?;

                // Retrieve the Vec<String> of all formats supported by the connected sensor
                let available_formats = pixel_format_node.settable_values()?;

                info!("Listing available pixel formats for this camera:");
                for format in available_formats {
                    info!(" - {}", format);
                }
                let mut was_tracking = false;
                let mut grab_result = pylon_cxx::GrabResult::new()?;

                camera.open()?;
                info!("Camera opened and idling. Waiting for TRACKING signal..."); 

                let frame_interval = Duration::from_millis(CAPTURE_RATE);
                let mut next_frame_time = Instant::now();

                camera.start_grabbing(&pylon_cxx::GrabOptions::default()
                    .strategy(pylon_cxx::GrabStrategy::LatestImageOnly))?;

                let mut darkframe_source: Vec<ImageBuffer<Luma<u8>, Vec<u8>>> = vec![];

                for _ in 0..20 {
                    match camera.retrieve_result(500, &mut grab_result, pylon_cxx::TimeoutHandling::Return) {
                        Ok(true) if grab_result.grab_succeeded().unwrap_or(false) =>
                        {
                            let raw_buffer: &[u8] = grab_result.buffer()?;
                            let width = grab_result.width()?;
                            let height = grab_result.height()?;

                            darkframe_source.push(ImageBuffer::from_raw(width, height, raw_buffer.to_vec())
                                .expect("Buffer size mismatch"));
                        }
                        _ => {
                            error!("Timeout or grab fail");
                        }
                    }
                }
                
                let avger = ImageAveragerFromBuffer::new_with_source(darkframe_source);

                if let Err(e) = avger.get_average().save(format!("{STAR_TRACKER_DIR}/dark_frame.tiff")) {
                    error!("Dark frame image save error, bad directory");
                }



                camera.stop_grabbing()?;

                // Cam loop
                loop {
                    let is_tracking = TRACKING.load(Ordering::Relaxed);

                    // Handle camera start/stop
                    if is_tracking && !was_tracking {
                        info!("Tracking on, start grabbing");
                        camera.start_grabbing(&pylon_cxx::GrabOptions::default()
                            .strategy(pylon_cxx::GrabStrategy::LatestImageOnly))?;
                        was_tracking = true;
                        next_frame_time = Instant::now() + frame_interval; // Initialize metronome
                    } 
                    else if !is_tracking && was_tracking {
                        info!("Tracking disabled. Safely stop grabbing");
                        camera.stop_grabbing()?;
                        was_tracking = false;
                    }

                    if is_tracking {
                        if camera.is_grabbing() {
                            
                            // Set the next time we'll take a picture now
                            // and adjust later based off of how much
                            // time spent on computation
                            next_frame_time += frame_interval;

                            match camera.retrieve_result(500, &mut grab_result, pylon_cxx::TimeoutHandling::Return) {
                                Ok(true) if grab_result.grab_succeeded().unwrap_or(false) => {
                                    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
                                    
                                    let raw_buffer: &[u8] = grab_result.buffer()?;
                                    let width = grab_result.width()?;
                                    let height = grab_result.height()?;

                                    // Copy
                                    let img_vec = raw_buffer.to_vec(); 
                                    let mut solve_img = ImageBuffer::from_raw(width, height, img_vec)
                                        .expect("Buffer size mismatch");

                                    avger.apply_average(&mut solve_img);

                                    // Try sending an image to be solved
                                    match solver_tx.try_send((timestamp, solve_img)) {
                                        Ok(_) => {
                                            // Wait for the solver up to 600ms leaving 400ms buffer for save and sleep
                                            // May want to adjust to handle initial case and then switch to tracking mode
                                            // But infratracker particularly has to deal with large rotations
                                            // so it's likely it will just have to stay in LOST IN SPACE mode 
                                            // the entire times
                                            while let Ok((ret_stamp, Some(quaternion))) = result_rx.try_recv() {
                                                self.send_packet(timestamp, quaternion);
                                            }
                                        }
                                        Err(TrySendError::Full(_)) => {
                                            error!("Solver thread hung, Skipping telemetry to save image.");
                                        }
                                        Err(TrySendError::Disconnected(_)) => {
                                            error!("Solver thread dead");
                                        }
                                    }

                                    // Do file save with zero copy
                                    // cuz we can get away with it
                                    let local_img: ImageBuffer<Luma<u8>, &[u8]> = 
                                        ImageBuffer::from_raw(width, height, raw_buffer).unwrap();
                                    
                                    if let Err(e) = local_img.save(format!("{STAR_TRACKER_DIR}/infratracker{timestamp}.tiff")) {
                                        error!("Image save error, bad directory")
                                    }
                                }
                                _ => {
                                    error!("Timeout or grab fail");
                                }
                            }
                        }

                        let now = Instant::now();
                        if next_frame_time > now {
                            thread::sleep(next_frame_time - now);
                        } else {
                            // Startracking and saving took longer than 1 second
                            // so immediately solve next frame
                            next_frame_time = now;
                        }
                    } 
                    else {
                        // Idle loop
                        thread::sleep(Duration::from_millis(200)); 
                    }
                }
                #[allow(unreachable_code)]
                Ok(())
            })();
            
            if let Err(thread_error) = result {
                error!("Error in running infratracker task: {thread_error}")
            }
        })
    }

    fn solve_attitude(
        img: &ImageBuffer<Luma<u8>, Vec<u8>>, 
        finder: &Starfinder, 
        model: &CameraModel, 
        solver: &Startracker
    ) -> Option<Quaternion<f32, ICRF<f32>, Body<f32>>> {
        let mut centroids = finder.star_find(img);
        model.undistort_centroids(&mut centroids);
        
        match solver.adaptive_pyramid_solve(centroids) {
            Ok((refs, body)) => Some(quest_real(&refs, &body)),
            Err(e) => {
                error!("Pyramid solve failed: {}", e);
                None
            }
        }
    }

    fn send_packet(&self, timestamp: u64, q: Quaternion<f32, ICRF<f32>, Body<f32>>) {
        let packet = ApplicationPacket::InfratrackerData { 
            timestamp,
            quaternion: [q.w(), q.i(), q.j(), q.k()],
        };
        if let Err(e) = self.quaternion_sender.send(packet) {
            error!("Error sending estimate: {}", e);
        }
    }


}