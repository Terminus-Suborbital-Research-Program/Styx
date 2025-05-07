
#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]


use core::{
    ptr::write_bytes,
    sync::atomic::{AtomicBool, Ordering},
};

use atmega_hal::{
    port::{
        self, 
        mode::{Floating, Input, Output},
        Dynamic, Pin, PA0,
    },
    prelude::_embedded_hal_serial_Read,
    usart::{Baudrate, Usart},
    Pins,
    clock::MHz16,
};

use embedded_hal::{delay::DelayNs, digital::InputPin};

use bin_packets::{data::PinState, ApplicationPacket, CommandPacket};
use bincode::{
    config::standard, 
    decode_from_slice, 
    encode_into_slice, 
    error::{DecodeError, EncodeError},
};

use i2c_slave::*;
use panic_halt as _;
use ufmt::{uwrite, uwriteln};

type CoreClock = MHz16;
type Delay = atmega_hal::delay::Delay<CoreClock>;
use heapless::Vec;


mod i2c_slave;

static TWI_INT_FLAG: AtomicBool = AtomicBool::new(false);

// I2C interrupt handler
#[avr_device::interrupt(atmega2560)]
fn TWI() {
    avr_device::interrupt::free(|_| {
        TWI_INT_FLAG.store(true, Ordering::SeqCst);
    });
}

trait Read {
    fn new(pin_set: &[Pin<Input<Floating>, Dynamic>; 7]) -> Self ;
    fn update(&mut self, pin_set: &[Pin<Input<Floating>, Dynamic>; 7]);
}

impl Read for PinState {
    fn new(pin_set: &[Pin<Input<Floating>, Dynamic>; 7]) -> Self {
        let state: Vec<bool, 7> = pin_set.iter().map(|pin| pin.is_high()).collect();

        PinState {
            gse_1: state[0],
            gse_2: state[1],
            te_ra: state[2],
            te_rb: state[3],
            te_1: state[4],
            te_2: state[5],
            te_3: state[6],
        }
    }

    fn update(&mut self, pin_set: &[Pin<Input<Floating>, Dynamic>; 7]) {
        let state: Vec<bool, 7> = pin_set.iter().map(|pin| pin.is_high()).collect();
        self.gse_1 =  state[0];
        self.gse_2 = state[1];
        self.te_ra = state[2];
        self.te_rb = state[3];
        self.te_1 = state[4];
        self.te_2 = state[5];
        self.te_3 = state[6];
    }
}


#[avr_device::entry]
fn main() -> ! {
    let dp = atmega_hal::Peripherals::take().unwrap();
    let pins = atmega_hal::pins!(dp);

    let mut serial = Usart::new(
        dp.USART0,
        pins.pe0,
        pins.pe1.into_output(),
        Baudrate::<crate::CoreClock>::new(57600),
    );

    let mut led = pins.pb7.into_output();

    let mut battery_latch = pins.pa7.into_output();

    let pin_set = [
        pins.pa0.into_floating_input().downgrade(),
        pins.pa1.into_floating_input().downgrade(),
        pins.pa2.into_floating_input().downgrade(),
        pins.pa3.into_floating_input().downgrade(),
        pins.pa4.into_floating_input().downgrade(),
        pins.pa5.into_floating_input().downgrade(),
        pins.pa6.into_floating_input().downgrade(),
    ];

    // Using external pullup resistors, so pins configured as floating inputWs
    let sda = pins.pd1.into_floating_input();
    let scl = pins.pd0.into_floating_input();
    
    let slave_address: u8 = 0x26;

    let mut i2c_slave: I2cSlave = I2cSlave::new(dp.TWI, slave_address, sda, scl, &TWI_INT_FLAG);

    // Enable global interrupt
    unsafe { avr_device::interrupt::enable() };
    // Value recieved from I2C Master
    let mut buf: [u8; 20];

    uwriteln!(&mut serial, "Initialized with addr: 0x{:X}", slave_address).ok();
    i2c_slave.init(false);

    led.set_low();


    // Check in and out of loop
    
    let mut read_buf: [u8; 20] = [0u8; 20];


    let mut byte = 0b0000_0000u8;

    let setter = 0b0000_0001;
    let mut pin_state = PinState::new(&pin_set);
    let mut write_buf: [u8; 1] = [0u8; 1];
    let mut read_buf: [u8; 1] = [0u8; 1];

    loop {

        pin_state.update(&pin_set);

        // byte &= !0b0000_0011;
        // byte &= 0b1111_1100;

        if pin_state.gse_1 {
            // byte |=  0b0000_0001;
            byte |=  setter;
        }
        if pin_state.gse_2 {
            // byte |=  0b0000_0010;
            byte |=  setter << 1;
        }
        if pin_state.te_ra {
            // byte |= 0b0000_0100;
            byte |= setter << 2;
        }
        if pin_state.te_rb {
            // byte |= 0b0000_1000;
            byte |= setter << 3;
        }
        if pin_state.te_1 {
            // byte |= 0b0001_0000;
            byte |= setter << 4;
        }
        if pin_state.te_2 {
            // byte |= 0b0010_0000;
            byte |= setter << 5;
        }
        if pin_state.te_3 {
            // byte |= 0b0100_0000;
            byte |= setter << 6;
        }
        
        write_buf[0] = byte;

        match i2c_slave.respond(&write_buf) {
            Ok(bytes_sent) => {
                uwriteln!(serial,"{} bytes sent", bytes_sent).ok();
            }
                    
            Err(err) => {
                uwriteln!(serial, "response_error").ok();
            }
        }

        match i2c_slave.receive(&mut read_buf) {
            Ok(bytes_read) => {
                uwriteln!(serial,"Succesful read").ok();
                if read_buf[0] == 1 {
                    battery_latch.set_high();
                } else {
                    battery_latch.set_low();
                }
            }
            Err(err) => {
                uwriteln!(serial, "Error: {:?}", err).ok();
            }
        }
        
        read_buf.fill(0);
    }
}