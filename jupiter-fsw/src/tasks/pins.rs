use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use i2cdev::{core::I2CDevice as _, linux::LinuxI2CDevice};
use log::{info, warn};

pub mod bitmaps {
    pub const GSE1_MASK: u8 = 0b00000001;
    pub const GSE2_MASK: u8 = 0b00000010;
    pub const TE_RA_MASK: u8 = 0b00000100;
    pub const TE_RB_MASK: u8 = 0b00001000;
    pub const TE1_MASK: u8 = 0b00010000;
    pub const TE2_MASK: u8 = 0b00100000;
    pub const TE3_MASK: u8 = 0b01000000;
}

use bitmaps::*;

#[derive(Default, Debug, Clone)]
pub struct PinStates {
    gse_1: bool,
    te_1: bool,
    te_2: bool,
    gse_2: bool,
    te_3: bool,
    te_ra: bool,
    te_rb: bool,
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

    // new abstraction to create PinStates from raw byte
    pub fn from_byte(byte: u8) -> Self {
        Self {
            gse_1: byte & GSE1_MASK != 0,
            te_1: byte & TE1_MASK != 0,
            te_2: byte & TE2_MASK != 0,
            gse_2: byte & GSE2_MASK != 0,
            te_3: byte & TE3_MASK != 0,
            te_ra: byte & TE_RA_MASK != 0,
            te_rb: byte & TE_RB_MASK != 0,
        }
    }

    // helper to update in-place
    pub fn update_from_byte(&mut self, byte: u8) {
        *self = Self::from_byte(byte);
    }
}

// new shared concurrent PinStates handle
#[derive(Clone)]
pub struct SharedPinStates(Arc<RwLock<PinStates>>);

impl SharedPinStates {
    pub fn new() -> Self {
        SharedPinStates(Arc::new(RwLock::new(PinStates::new())))
    }

    pub fn read(&self) -> PinStates {
        self.0.read().map(|p| p.clone()).unwrap_or_default()
    }

    pub fn update(&self, byte: u8) {
        if let Ok(mut w) = self.0.write() {
            w.update_from_byte(byte);
        } else {
            warn!("Failed to acquire write lock on PinStates");
        }
    }
}

// update signature to use SharedPinStates and simplify body
pub fn pin_states_thread(mut atmega: LinuxI2CDevice, pin_states: SharedPinStates) -> ! {
    loop {
        let bytes = match atmega.smbus_read_byte() {
            Ok(b) => b,
            Err(e) => {
                warn!("Error reading ATMEGA: {e:?}");
                0
            }
        };

        // update shared state
        pin_states.update(bytes);
        // log current state
        let states = pin_states.read();
        info!("Pin states: {states:?}");

        std::thread::sleep(Duration::from_millis(1000));
    }
}
