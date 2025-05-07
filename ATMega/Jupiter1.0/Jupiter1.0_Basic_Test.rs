
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


fn delay_ms(ms: u16) {
    Delay::new().delay_ms(u32::from(ms))
}

#[allow(dead_code)]
fn delay_us(us: u32) {
    Delay::new().delay_us(us)
}

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


    let TE_1 = pins.pa4.into_floating_input();
    let GSE_1 = pins.pa0.into_floating_input();


    //let mut pin_state = PinState::new(pin_set);
    // Using external pullup resistors, so pins configured as floating inputWs
    let sda = pins.pd1.into_floating_input();
    let scl = pins.pd0.into_floating_input();
    
    let slave_address: u8 = 0x26;

    let mut i2c_slave: I2cSlave = I2cSlave::new(dp.TWI, slave_address, sda, scl, &TWI_INT_FLAG);

    // Enable global interrupt
    unsafe { avr_device::interrupt::enable() };

    // Disabling power reduction for TWI

    // Value recieved from I2C Master
    let mut buf: [u8; 20];

    uwriteln!(&mut serial, "Initialized with addr: 0x{:X}", slave_address).ok();
    i2c_slave.init(false);

    led.set_low();


    // Check in and out of loop
    
    let mut read_buf: [u8; 20] = [0u8; 20];


    let mut byte = 0b0000_0000u8;

    loop {
        let mut write_buf: [u8; 1] = [0u8; 1];

        byte &= 0b0000_0011;
       
        if TE_1.is_high() {
            byte |= 0b0000_0010;
        }
        if GSE_1.is_high() {
            byte |=  0b0000_0001;
        }

        write_buf[0] = byte;

        match i2c_slave.respond(&write_buf) {
            Ok(bytes_sent) => {
                uwriteln!(serial,"{} bytes sent", bytes_sent).ok()
            }
                    
            Err(err) => {
                uwriteln!(serial, "response_error").ok()
            }
        }

    }
}


// #[avr_device::interrupt(atmega2560)]
// fn TIMER1_COMPA() {
//     let state = unsafe {
//         // SAFETY: We _know_ that interrupts will only be enabled after the LED global was
//         // initialized so this ISR will never run when LED is uninitialized.
//         &mut *INTERRUPT_STATE.as_mut_ptr()
//     };

//     state.blinker.toggle();
// }