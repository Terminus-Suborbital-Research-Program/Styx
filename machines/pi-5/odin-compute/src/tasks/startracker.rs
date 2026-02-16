use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;

use v4l::Device;
use v4l::io::mmap::Stream;

use v4l::io::mmap::Stream as MmapStream;
// use v4l::prelude::*;
use v4l::buffer::Metadata;

use image::ImageBuffer;
use v4l::video::Capture;

use image::Luma;
use wayfarer::startrack::quest::quest_real;

use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle};

use wayfarer::{
    perception::{
    camera_model::CameraModel,
    centroiding::Starfinder,
    },
    startrack::{
        solver::Startracker,
        quest::quest,
    },
};// pub use crate::io::mmap::Stream as MmapStream;
use aether::attitude::Quaternion;
use aether::reference_frame::{
    Body,
    ICRF
};
use log::{error, info, LevelFilter};

// use aether::
pub struct StartrackerThread{
    quaternion_sender: Sender<Quaternion<f32, ICRF<f32>,Body<f32>>>,
    // Sender<Quaternion<f64, ICRF<f64>,Body<f64>>>
}

impl StartrackerThread {

    pub fn new() -> (Self, Receiver<Quaternion<f32, ICRF<f32>,Body<f32>>>) {
        let (quaternion_tx, quaternion_rx) = channel();

        let startracker = Self {
            quaternion_sender: quaternion_tx,
        };

        (startracker, quaternion_rx)
    }
    
    pub fn startrack(self)  -> JoinHandle<()> {

        let mut dev = Device::new(0).expect("Failed to open device");
        let fmt = dev.format().expect("Failed to read format");

        let height= fmt.height;
        let width = fmt.width;

        let mut stream: MmapStream =
            MmapStream::with_buffers(&mut dev, Type::VideoCapture, 4).expect("Failed to create buffer stream");
        
        let starfinder = Starfinder::default();
        let camera_model = CameraModel::default();
        let startracker = Startracker::default();

        let thread = thread::spawn(move || {

            

            loop {
                let (buf, meta): (&[u8], &Metadata) = stream.next().expect("Failed to get frame");

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

                let mut img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, buffer)
                    .expect("Buffer size mismatch");
                
                let mut centroids = starfinder.star_find(&mut img);
                camera_model.undistort_centroids(&mut centroids);
                match startracker.pyramid_solve(centroids) {
                // match startracker.exhaustive_solve(centroids, 100) {
                    Ok((reference_vectors, body_vectors)) => {
                        let q: Quaternion<f32, ICRF<f32>,Body<f32>>  = quest_real(&reference_vectors, &body_vectors);

                         if let Err(e) = self.quaternion_sender.send(q) {
                            error!("Error sending estimate: {}", e);
                        }
                    }

                    Err(e) => {
                        error!("{}",e);
                    }
                }
            }
        });
        thread
    }
}