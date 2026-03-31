//! SD Card management for the Ejector  

#[warn(missing_docs)]

use bincode::de;
use defmt::info;
use embedded_hal::{delay::DelayNs, digital::OutputPin, spi::SpiDevice as EmbHal_SpiDevice};
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{Directory, Mode, SdCard, TimeSource, Timestamp, VolumeIdx, VolumeManager};
use heapless::String;
use rp235x_hal::{
    gpio::{
        bank0::{Gpio16, Gpio17, Gpio18, Gpio19},
        Function, FunctionSio, FunctionSpi, Pin, PullDown, SioOutput,
    },
    pac::RESETS,
    spi::{Enabled, SpiDevice},
    timer::CopyableTimer0,
    Spi, Timer,
};

pub const EJECTOR_SD_FILENAME: &'static str = "data.txt";

/// Struct to manage the SD card on the Ejector. This is mostly 
/// a wrapper around the embedded_sdmmc crate, which provides a 
/// high-level API for managing SD cards.
pub struct EjectorSdCard2<MosiPin, MisoPin, ClkPin, CsPin, SpiBus, Timer>
where
    MosiPin: OutputPin + Function,
    MisoPin: OutputPin + Function,
    ClkPin: OutputPin + Function,
    CsPin: OutputPin + Function,
    SpiBus: SpiDevice + EmbHal_SpiDevice,
    Timer: DelayNs,
{
    phantom: core::marker::PhantomData<(MosiPin, MisoPin, ClkPin, CsPin, SpiBus, Timer)>,
    sdcard: SdCard<SpiBus, Timer>,
    //root_dir: Directory<'_, SdCard<SpiBus, Timer>, Timer, DummyTimesource, 4, 4>,
}

#[derive(Default)]
pub struct DummyTimesource();

impl TimeSource for DummyTimesource {
    // In theory you could use the RTC of the rp2040 here, if you had
    // any external time synchronizing device.
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

impl<MosiPin, MisoPin, ClkPin, CsPin, SpiBus, Timer>
    EjectorSdCard2<MosiPin, MisoPin, ClkPin, CsPin, SpiBus, Timer>
where
    MosiPin: OutputPin + Function,
    MisoPin: OutputPin + Function,
    ClkPin: OutputPin + Function,
    CsPin: OutputPin + Function,
    SpiBus: SpiDevice + EmbHal_SpiDevice,
    Timer: DelayNs,
{
    pub fn new(spi_bus: SpiBus, clock_sourc: Timer) -> () {
        let sdcard = SdCard::new(spi_bus, clock_sourc);

        //let mut t = Directory::<'_, SdCard<SpiBus, Timer>, DummyTimesource, 4, 4, 1>::default();

        let mut volume_mgr = VolumeManager::new(sdcard, DummyTimesource::default());
        match volume_mgr.open_volume(VolumeIdx(0)) {
            Ok(volume) => match volume.open_root_dir() {
                Ok(root_dir) => {
                    let mut t = root_dir
                        .open_file_in_dir(EJECTOR_SD_FILENAME, Mode::ReadWriteCreateOrTruncate);
                }
                Err(e) => info!("Failed to open root directory: "),
            },
            Err(e) => info!("Failed to open volume:"),
        };
    }
}

pub fn spi_bus() -> () {}
