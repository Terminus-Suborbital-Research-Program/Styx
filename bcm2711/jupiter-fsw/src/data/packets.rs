use std::{
    ffi::OsStr,
    fs::File,
    path::{Path, PathBuf},
};

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
        std::process::Command::new("mkdir")
            .arg(dir_path.clone())
            .arg("-p")
            .output()
            .unwrap();

        let path = Path::new(&dir_path);

        let new_name = std::fs::read_dir(path)
            .unwrap()
            .map(|x| x.ok())
            .filter(|x| x.is_some())
            .map(|x| x.unwrap())
            .map(|x| {
                x.path()
                    .file_name()
                    .unwrap_or(OsStr::new("0"))
                    .to_string_lossy()
                    .parse::<u32>()
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0)
            + 1;

        let mut path = PathBuf::from(path);
        path.push(format! {"{}", new_name});
        let file = File::create(path).unwrap();

        Self::new(file)
    }
}

