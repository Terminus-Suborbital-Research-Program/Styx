use defmt::info;
use embedded_hal::{delay::DelayNs, spi::SpiDevice as EmbHal_SpiDevice};
use embedded_hal::digital::OutputPin;
use embedded_hal_bus::spi::{ExclusiveDevice};
use embedded_sdmmc::{Directory, Mode, SdCard, TimeSource, Timestamp, VolumeIdx, VolumeManager};
use heapless::String;
use rp235x_hal::gpio::bank0::{Gpio16, Gpio17, Gpio18, Gpio19};
use rp235x_hal::gpio::Pin;
use rp235x_hal::gpio::{Function, FunctionSio};
use rp235x_hal::spi::SpiDevice;
use rp235x_hal::{
    gpio::{FunctionSpi, PullDown, SioOutput},
    pac::RESETS,
    spi::Enabled,
    timer::CopyableTimer0,
    Spi, Timer,
};

//pub type MOSI = Pin<Gpio19, FunctionSpi, PullDown>;
//pub type MISO = Pin<Gpio16, FunctionSpi, PullDown>;
//pub type CLK = Pin<Gpio18, FunctionSpi, PullDown>;
//pub type CS = Pin<Gpio17, FunctionSio<SioOutput>, PullDown>;
//pub type SpiBus = Spi<Enabled, impl SpiDevice, (MOSI, MISO, CLK), 8>;
//
pub const EJECTOR_SD_FILENAME: &'static str = "data.txt";
//
//pub struct EjectorSdCard {
//    sdcard: SdCard<SpiBus, Timer<CopyableTimer0>>,
//    root_dir: Directory<
//        '_,
//        SdCard<SpiBus, Timer<CopyableTimer0>>,
//        Timer<CopyableTimer0>,
//        DummyTimesource,
//        4,
//        4,
//        1,
//    >,
//}

pub struct EjectorSdCard2<MosiPin, MisoPin, ClkPin, CsPin, SpiBus, Timer>
where
    MosiPin: OutputPin + Function,
    MisoPin: OutputPin + Function,
    ClkPin: OutputPin + Function,
    CsPin: OutputPin + Function,
    SpiBus: SpiDevice + EmbHal_SpiDevice,
    Timer: DelayNs,
{
    sdcard: SdCard<SpiBus, Timer>,
    root_dir: Directory<'_, SdCard<SpiBus, Timer>, Timer, DummyTimesource, 4, 4>,
    
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

//impl EjectorSdCard {
//    pub fn new(spi_bus: SpiBus, clock_sourc: Timer<CopyableTimer0>) -> () {
//        let sdcard = SdCard::new(spi_bus, clock_sourc);
//        let mut t = Default::default();
//        let mut volume_mgr = VolumeManager::new(sdcard, DummyTimesource::default());
//        match volume_mgr.open_volume(VolumeIdx(0)) {
//            Ok(volume) => match volume.open_root_dir() {
//                Ok(root_dir) => {
//                    t = root_dir
//                        .open_file_in_dir(EJECTOR_SD_FILENAME, Mode::ReadWriteCreateOrTruncate);
//                }
//                Err(e) => info!("Failed to open root directory: {:?}", e),
//            },
//            Err(e) => info!("Failed to open volume: {:?}", e),
//        }
//    }
//}

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
        let mut t = Default::default();
        let mut volume_mgr = VolumeManager::new(sdcard, DummyTimesource::default());
        match volume_mgr.open_volume(VolumeIdx(0)) {
            Ok(volume) => match volume.open_root_dir() {
                Ok(root_dir) => {
                    t = root_dir
                        .open_file_in_dir(EJECTOR_SD_FILENAME, Mode::ReadWriteCreateOrTruncate);
                }
                Err(e) => info!("Failed to open root directory: {:#?}", e),
            },
            Err(e) => info!("Failed to open volume: {:#?}", e),
        }
    }
}

pub fn spi_bus() -> () {}
