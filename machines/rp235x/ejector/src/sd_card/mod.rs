//! SD Card management for the Ejector  

#![warn(missing_docs, clippy::unwrap_used)]
use bincode::{de, Decode};
    use bin_packets::time::Timestamp;

use defmt::info;
use embedded_hal::{delay::DelayNs, digital::OutputPin, spi::SpiDevice};
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{Directory, Mode, SdCard, Timestamp as SdTimestamp, TimeSource, VolumeIdx, VolumeManager};
use heapless::String;
use rp235x_hal::{
    gpio::{Function, FunctionSio, FunctionSpi, Pin, PullDown, SioOutput},
    pac::RESETS,
    spi::Enabled,
    timer::CopyableTimer0,
    Spi, Timer,
};

pub const THERMAL_COAT_FILENAME: &'static str = "thermalCoatData.txt";
pub const GAURD_FILENAME: &'static str = "guardThermalData.txt";
pub const BMM_FILENAME: &'static str = "bmmData.txt";

/// Struct to manage the SD card on the Ejector. This is mostly
/// a wrapper around the embedded_sdmmc crate, which provides a
/// high-level API for managing SD cards.
pub struct EjectorSdCard<SpiBus, Timer>
where
    SpiBus: SpiDevice,
    Timer: DelayNs,
{
    //phantom: core::marker::PhantomData<(SpiBus, Timer)>,
    vol: VolumeManager<SdCard<SpiBus, Timer>, DummyTimesource>,
    //root_dir: Directory<'_, SdCard<SpiBus, Timer>, Timer, DummyTimesource, 4, 4>,
}

#[derive(Default)]
pub struct DummyTimesource();

impl TimeSource for DummyTimesource {
    // In theory you could use the RTC of the rp2040 here, if you had
    // any external time synchronizing device.
    fn get_timestamp(&self) -> SdTimestamp {
        SdTimestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

impl<SpiBus, Timer> EjectorSdCard<SpiBus, Timer>
where
    SpiBus: SpiDevice,
    Timer: DelayNs,
{
    pub fn new(spi_bus: SpiBus, clock_sourc: Timer) -> Self {
        let sdcard = SdCard::new(spi_bus, clock_sourc);

        //let mut volume_mgr = VolumeManager::new(sdcard, DummyTimesource::default());
        //match volume_mgr.open_volume(VolumeIdx(0)) {
        //    Ok(volume) => match volume.open_root_dir() {
        //        Ok(root_dir) => {
        //            let mut t = root_dir
        //                .open_file_in_dir(EJECTOR_Z_DATA_FILENAME, Mode::ReadWriteCreateOrTruncate);
        //
        //        }
        //        Err(e) => info!("Failed to open root directory: "),
        //    },
        //    Err(e) => info!("Failed to open volume:"),
        //};
        Self {
            vol: VolumeManager::new(sdcard, DummyTimesource::default()),
        }
    }

    pub fn write_data(&mut self, file_name: &'static str, data: &[u8]) -> Result<(), ()> {
        //let mut volume_mgr = VolumeManager::new(self.sdcard, DummyTimesource::default());
        let t = match self.vol.open_volume(VolumeIdx(0)) {
            Ok(mut volume) => match volume.open_root_dir() {
                Ok(mut root_dir) => {
                    let mut file = root_dir
                        .open_file_in_dir(file_name, Mode::ReadWriteCreateOrTruncate)
                        .map_err(|_| ())?;
                    file.write(data).map_err(|_| ())?;
                    Ok(())
                }
                Err(_) => Err(()),
            },
            Err(_) => Err(()),
        };
        return t;
    }
}

pub struct ThermoData {
    pub thermo_data: Option<(Timestamp, f32, f32, f32)>,
}

pub struct CoatingData {
    pub z_data: Option<(Timestamp, f32)>,
}

pub struct BmmData {
    pub bmm_data: Option<(Timestamp, f32)>,
}

#[derive(Encode, Decode)]
pub struct SdData {
    pub thermo_data: Option<(Timestamp, f32, f32, f32)>,
    pub coating_data: Option<(Timestamp, f32)>,
    pub bmm_data: Option<(Timestamp, f32)>,
}

impl Default for SdData {
    fn default() -> Self {
        Self {
            thermo_data: Default::default(),
            coating_data: Default::default(),
            bmm_data: Default::default(),
        }
    }
}
