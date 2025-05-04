use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use i2cdev::{core::I2CDevice as _, linux::LinuxI2CDevice};
use log::{info, warn};

#[derive(Default)]
pub struct PinStates {
    gse_1: bool,
    te_1: bool,
    te_2: bool, //added te2

    //unused, future proofing
    gse_2: bool,
    te_3: bool,
    te_ra: bool,
    te_rb: bool,
    /////////////
}

#[allow(dead_code)]
impl PinStates {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn gse_1_high(&self) -> bool {
        self.gse_1
    }

    pub fn te_1_high(&self) -> bool {
        self.te_1
    }

    pub fn te_2_high(&self) -> bool {
        self.te_2
    }

    pub fn set_pins(
        &mut self,
        gse_1: bool,
        te_1: bool,
        te_2: bool,
        gse_2: bool,
        te_3: bool,
        te_ra: bool,
        te_rb: bool,
    ) {
        self.gse_1 = gse_1;
        self.te_1 = te_1;
        self.te_2 = te_2;

        //unused, future proofing
        self.gse_2 = gse_2;
        self.te_3 = te_3;
        self.te_ra = te_ra;
        self.te_rb = te_rb;
        /////////////
    }

    //unused, future proofing
    pub fn gse_2_high(&self) -> bool {
        self.gse_2
    }

    pub fn te_3_high(&self) -> bool {
        self.te_3
    }

    pub fn te_ra_high(&self) -> bool {
        self.te_ra
    }

    pub fn te_rb_high(&self) -> bool {
        self.te_rb
    }
    /////////////
}

// Divirging function to handle reading from the pins
pub fn pin_states_thread(mut atmega: LinuxI2CDevice, pin_states: Arc<RwLock<PinStates>>) -> ! {
    loop {
        let bytes = match atmega.smbus_read_byte() {
            Ok(b) => b,
            Err(e) => {
                warn!("Error reading ATMEGA: {:?}", e);
                0
            }
        };

        let gse1 = 1u8 & bytes == 1u8; // Current
        let gse2 = 0b10u8 & bytes == 0b10u8;

        let tera = 0b100u8 & bytes == 0b100u8;
        let terb = 0b1000u8 & bytes == 0b1000u8;

        let te1 = 0b1_0000u8 & bytes == 0b1_0000u8; // Current
        let te2 = 0b10_0000u8 & bytes == 0b10_0000u8; // Current
        let te3 = 0b100_0000u8 & bytes == 0b100_0000u8;
        info!(
            " GSE-1: {}, GSE-2: {}, TE-RA: {}, TE-RB: {}, TE-1: {}, TE-2{}, TE-3{}",
            gse1, gse2, tera, terb, te1, te2, te3
        );

        match pin_states.write() {
            Ok(mut writer) => {
                writer.set_pins(gse1, te1, te2, gse2, te3, tera, terb);
            }

            Err(e) => {
                warn!("Error getting writer! Error: {:?}", e);
            }
        }

        std::thread::sleep(Duration::from_millis(1000));
    }
}
