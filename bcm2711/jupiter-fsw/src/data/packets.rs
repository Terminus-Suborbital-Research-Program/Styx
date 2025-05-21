use std::{
    ffi::OsStr,
    fs::File,
    path::{Path, PathBuf},
};

use log::info;

use bin_packets::{
    device::{PacketWriter, std::Device},
    packets::ApplicationPacket,
};

pub struct OnboardPacketStorage {
    file: Device<File>,
}

impl OnboardPacketStorage {
    fn new(file: File) -> Self {
        Self {
            file: Device::new(file),
        }
    }

    pub fn write<T: Into<ApplicationPacket>>(&mut self, packet: T) {
        self.file.write(packet.into()).ok();
    }

    pub fn get_current_run() -> Self {
        let home = std::env::var("HOME").expect("No $HOME variable? What the fuck?");
        let dir_path = format!("{}/data/packets", home);
        // Get the directory if it doesn't exist
        info!("Making directory...");
        std::process::Command::new("mkdir")
            .arg(dir_path.clone())
            .arg("-p")
            .spawn()
            .unwrap()
            .wait()
            .ok();

        let path = Path::new(&dir_path);

        info!("Finding new name...");
        let mut max = 0;
        for dir in std::fs::read_dir(path).unwrap() {
            if let Ok(name) = dir {
                info!("Found {}", name.file_name().to_string_lossy());
                let x = name
                    .file_name()
                    .to_string_lossy()
                    .parse::<u32>()
                    .unwrap_or(0);
                if x > max {
                    max = x;
                }
            }
        }
        let mut path = PathBuf::from(path);
        path.push(format! {"{}", max + 1});
        let file = File::create(path).unwrap();

        Self::new(file)
    }
}
