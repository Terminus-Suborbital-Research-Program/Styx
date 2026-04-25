use std::fs::create_dir;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvError, SendError, Sender};
use std::thread::{self, sleep, JoinHandle};
use std::time::{Duration, SystemTime};

use bin_packets::packets::ApplicationPacket;
use lazy_static::lazy_static;
use log::info;
use std::time::UNIX_EPOCH;

use log::error;

use image::{ImageBuffer, Luma};

use wayfarer::perception::centroiding::Starfinder;
use wayfarer::perception::camera_model::CameraModel;
use wayfarer::startrack::solver::Startracker;
use wayfarer::startrack::quest::quest_real;

use aether::attitude::Quaternion;
use aether::reference_frame::{ICRF, Body};

const COUNT_IMAGES_TO_GRAB: u32 = 100;
const STAR_TRACKER_DIR: &str = "/home/terminus/basler/";
static BUFFER_TIME_MS: u64 = 1000;

lazy_static! {
    pub static ref TRACKING: AtomicBool = AtomicBool::new(false);
}



pub struct InfratrackerThread {
    quaternion_sender: Sender<ApplicationPacket>,
}

impl InfratrackerThread {
    pub fn new() -> (Self, Receiver<ApplicationPacket>) {
        let (quaternion_tx, quaternion_rx) = channel();

        let startracker = Self {
            quaternion_sender: quaternion_tx,
        };

        (startracker, quaternion_rx)
    }

    pub fn begin_startracking(self) -> JoinHandle<()> {
        
        info!("Starting Basler camera!");
        create_dir(STAR_TRACKER_DIR).ok();
        
        thread::spawn(move || {
            let result: Result<(), Box<dyn std::error::Error>> = (|| {

                // Cam
                let pylon = pylon_cxx::Pylon::new();
                let camera = pylon_cxx::TlFactory::instance(&pylon).create_first_device()?;
                let mut was_tracking = false;
                let mut grab_result = pylon_cxx::GrabResult::new()?;

                camera.open()?;
                println!("Camera opened and idling. Waiting for TRACKING signal..."); 

                // Startracker
                let starfinder = Starfinder::default();
                // IMPORTANT : REPLACE THIS WITH BASLER CAM PARAMETERS BECAUSE DEFAULT IS TEVS
                let camera_model = CameraModel::default();
                let startracker = Startracker::default();

                               

                loop {

                    let is_tracking = TRACKING.load(Ordering::Relaxed);

                    // Handle opening and closing cam with two bools
                    // so that we do not try to restart grabbing every time
                    if is_tracking && !was_tracking {
                        info!("Tracking on, start grabbing");
                        camera.start_grabbing(&pylon_cxx::GrabOptions::default()
                            .strategy(pylon_cxx::GrabStrategy::LatestImageOnly))?;
                        was_tracking = true;
                    } 
                    else if !is_tracking && was_tracking {
                        info!("Tracking disabled. Safely stop grabbing");
                        camera.stop_grabbing()?;
                        was_tracking = false;
                    }

                    if is_tracking {
                        if camera.is_grabbing() {
                            match camera.retrieve_result(500, &mut grab_result, pylon_cxx::TimeoutHandling::Return) {
                                Ok(true) if grab_result.grab_succeeded().unwrap_or(false) => {
                                    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
                                    
                                    let raw_buffer: &[u8] = grab_result.buffer()?;
                                    let img: ImageBuffer<Luma<u8>, &[u8]> = ImageBuffer::from_raw(
                                        grab_result.width()?, grab_result.height()?, raw_buffer
                                    ).expect("Buffer size mismatch");

                                    if let Some(quaternion) = Self::solve_attitude(&img, &starfinder, &camera_model, &startracker) {
                                        self.send_packet(timestamp, quaternion);
                                    }
                                    img.save(format!("{STAR_TRACKER_DIR}/infratracker{timestamp}.tiff")).ok();
                                }
                                _ => {
                                    error!("Timeout or grab fail");
                                }
                            }
                        }
                        // Don't run as fast as possible so we don't overwhelm sd card with
                        // data.
                        thread::sleep(Duration::from_millis(200));
                    } 
                    else {
                        thread::sleep(Duration::from_millis(200)); 
                    }
                    
                }
            Ok(())
        })();
            if let Err(thread_error) = result {
                error!("Error in running infratracker task: {thread_error}")
            }
        })
    }

    fn solve_attitude(
        img: &ImageBuffer<Luma<u8>, &[u8]>, 
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