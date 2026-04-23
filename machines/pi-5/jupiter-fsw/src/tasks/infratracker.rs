const COUNT_IMAGES_TO_GRAB: u32 = 100;

use bin_packets::packets::ApplicationPacket;
use std::thread::{self, JoinHandle};
use std::sync::mpsc::{Sender, Receiver, SendError, RecvError, channel};
use std::time::SystemTime;



pub struct InfratrackerThread {
    quaternion_sender: Sender<ApplicationPacket>,
    // Sender<Quaternion<f64, ICRF<f64>,Body<f64>>>
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
        

        let starfinder = Starfinder::default();
        let camera_model = CameraModel::default();
        let startracker = Startracker::default();

        
        thread::spawn(move || {
            let result: Result<(), Box<dyn std::error::Error>> = (|| {
            // Before using any pylon methods, the pylon runtime must be initialized.
            let pylon = pylon_cxx::Pylon::new();

            // Create an instant camera object with the camera device found first.
            let camera = pylon_cxx::TlFactory::instance(&pylon).create_first_device()?;

            // Print the model name of the camera.
            println!("Using device {:?}", camera.device_info().model_name()?);

            camera.open()?;
          

            

            

            // camera.enum_node("PixelFormat")?.set_value("RGB8")?;

            // Start the grabbing of COUNT_IMAGES_TO_GRAB images.
            // The camera device is parameterized with a default configuration which
            // sets up free-running continuous acquisition.
            camera.start_grabbing(&pylon_cxx::GrabOptions::default().count(COUNT_IMAGES_TO_GRAB))?;

            match camera.node_map()?.enum_node("PixelFormat") {
                Ok(node) => println!(
                    "pixel format: {}",
                    node.value().unwrap_or("could not read value".to_string())
                ),
                Err(e) => eprintln!("Ignoring error getting PixelFormat node: {}", e),
            };

            let mut grab_result = pylon_cxx::GrabResult::new()?;

            // Camera.StopGrabbing() is called automatically by the RetrieveResult() method
            // when c_countOfImagesToGrab images have been retrieved.
            while camera.is_grabbing() {
                // Wait for an image and then retrieve it. A timeout of 5000 ms is used.
                camera.retrieve_result(
                    5000,
                    &mut grab_result,
                    pylon_cxx::TimeoutHandling::ThrowException,
                )?;

                // Image grabbed successfully?
                if grab_result.grab_succeeded()? {
                    // Access the image data.
                    println!("SizeX: {}", grab_result.width()?);
                    println!("SizeY: {}", grab_result.height()?);

                    let image_buffer = grab_result.buffer()?;
                    println!("Value of first pixel: {}\n", image_buffer[0]);
                } else {
                    println!(
                        "Error: {} {}",
                        grab_result.error_code()?,
                        grab_result.error_description()?
                    );
                }
            }

            // Looks like I cannot take the raw image buffer with v4l and mutate it, so will have to perform one copy.
            // This causes an issue based on the way I scan for centroids, where I blot out dead pixels as a pass through. Will
            // Retest and benchmark later to see if I can get away without doing that, and if so, then this isn't as much a worry.

            // Also look into using userptr buffers later on (we own)

            // println!(
            //     "Buffer size: {}, seq: {}, timestamp: {}",
            //     buf.len(),      // buf is a &[u8] slice
            //     meta.sequence,  // meta is v4l::buffer::Metadata
            //     meta.timestamp  // timestamp is v4l::buffer::Timestamp
            // );

            let buffer = buf.to_vec();

            let mut img: ImageBuffer<Luma<u8>, Vec<u8>> =
                ImageBuffer::from_raw(width, height, buffer).expect("Buffer size mismatch");

            let mut centroids = starfinder.star_find(&mut img);
            camera_model.undistort_centroids(&mut centroids);
            match startracker.adaptive_pyramid_solve(centroids) {
                Ok((reference_vectors, body_vectors)) => {

                    let q: Quaternion<f32, ICRF<f32>, Body<f32>> =
                        quest_real(&reference_vectors, &body_vectors);

                    let timestamp = SystemTime::now()
                                .duration_since(UNIX_EPOCH)?
                                .as_millis() as u64;

                    let infra_packet = ApplicationPacket::InfratrackerData { 
                        timestamp: timestamp,
                        quaternion: [
                            quaternion.w(),
                            quaternion.i(),
                            quaternion.j(),
                            quaternion.k(),
                        ],
                    };

                    if let Err(e) = self.quaternion_sender.send(infra_packet) {
                        error!("Error sending estimate: {}", e);
                    }
                }

                Err(e) => {
                    error!("{}", e);
                }
            }
            Ok(())
        })();
        })
    }
}