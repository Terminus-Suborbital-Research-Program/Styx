
#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use core::{
    ptr::write_bytes,
    sync::atomic::{AtomicBool, Ordering},
};


use atmega_hal::port::mode::{Floating, Input, Output};
use atmega_hal::port::{self, Pin, Dynamic};
use atmega_hal::usart::{Baudrate, Usart};
use atmega_hal::prelude::_embedded_hal_serial_Read;

use heapless::Vec;


use embedded_hal::{delay::DelayNs, digital::InputPin};

use bin_packets::{data::PinState, ApplicationPacket, CommandPacket};
use bincode::{
    config::standard, decode_from_slice, encode_into_slice, error::{DecodeError, EncodeError}
};


use i2c_slave::*;
use panic_halt as _;
use ufmt::{uwrite, uwriteln};

type CoreClock = atmega_hal::clock::MHz16;
type Delay = atmega_hal::delay::Delay<crate::CoreClock>;

mod i2c_slave;

static TWI_INT_FLAG: AtomicBool = AtomicBool::new(false);
static REQUEST: AtomicBool = AtomicBool::new(false);

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
}

    // fn iter(pin_state: PinState) -> Self::IntoIter;

pub struct PinStateIter {
    pin_state: PinState,
    index: usize,
}

impl Iterator for PinStateIter {

    type Item = bool;

    fn next() -> {

    }
}

// impl From<PinState> for u8 {
//     fn from(pinstate: PinState) -> u8 {
//         let mut state_u8 = 0;

        
//     }
// }

#[avr_device::entry]
fn main() -> ! {
    let dp = atmega_hal::Peripherals::take().unwrap();
    let pins = atmega_hal::pins!(dp);

    let pin_set = &[
        pins.pa0.into_floating_input().downgrade(),
        pins.pa1.into_floating_input().downgrade(),
        pins.pa2.into_floating_input().downgrade(),
        pins.pa3.into_floating_input().downgrade(),
        pins.pa4.into_floating_input().downgrade(),
        pins.pa5.into_floating_input().downgrade(),
        pins.pa6.into_floating_input().downgrade(),
    ];

    let mut serial = Usart::new(
        dp.USART0,
        pins.pe0,
        pins.pe1.into_output(),
        Baudrate::<crate::CoreClock>::new(57600),
    );

    
    let mut ejector_usart = Usart::new(
        dp.USART1, 
        pins.pd2, 
        pins.pd3.into_output(), 
        Baudrate::<crate::CoreClock>::new(9600)
    );

    let mut rocket_usart = Usart::new(
        dp.USART2, 
        pins.ph0, 
        pins.ph1.into_output(), 
        Baudrate::<crate::CoreClock>::new(9600)
    );

    let mut led = pins.pb7.into_output();

    // Using external pullup resistors, so pins configured as floating inputs
    let sda = pins.pd1.into_floating_input();
    let scl = pins.pd0.into_floating_input();

    let slave_address: u8 = 0x26;

    let mut i2c_slave: I2cSlave = I2cSlave::new(dp.TWI, slave_address, sda, scl, &TWI_INT_FLAG);

    // External Interrupt Control Register A modified to detect 
    dp.EXINT.eicra.modify(|_, w| w.isc3().bits(0x03)); // 1100_0000 For ISC31 and ISC30 rising edge trigger configuration
    dp.EXINT.eimsk.modify(|_, w| w.int().bits(0x04)); // 0000_0100 For Int3 enable. This also clears the entire register
                                                                // but this is fine when just using interrupt 3

    // Enable global interrupt
    unsafe { avr_device::interrupt::enable() };

    led.set_low();


    let mut read_buf: Vec<u8, 30> = Vec::new();

    let mut ejector_buf: Vec<u8, 30> = Vec::new();

    let mut write_buf: [u8; 20] = [0u8; 20];



    loop {
        if REQUEST.load(Ordering::SeqCst) {
            loop {
                match ejector_usart.read() {
                    Ok(byte) => {
                        ejector_buf.push(byte);
                        uwriteln!(serial, "Byte read: {}", byte).unwrap();
                    }
    
                    Err(nb::Error::WouldBlock) => { 
                            // turn off interrupt flag
                            // Send raw encoded packet to rocket
                        for byte in &ejector_buf {
                            uwrite!(rocket_usart,"{}", byte).unwrap();
                        }
                        REQUEST.store(false, Ordering::SeqCst);
                        break;
                    }
                    
                    Err(nb::Error::Other(_e)) => { 
                        continue; 
                    }
                }
            }
        } 
            
        let telemetry_packet: Result<(
            ApplicationPacket, usize), bincode::error::DecodeError>  = decode_from_slice(read_buf.as_slice(), standard());
        
        read_buf.clear();

            match request {
                Ok((app_packet, len)) => {
                    match app_packet {
                        ApplicationPacket::Command(CommandPacket::Ping) => {
                                ufmt::uwriteln!(serial, "App Packet Ping found").unwrap();
                                request_success = true;
                            }
                            _ => {
                                // Send packets that are not a ping to rocket
                                for byte in read_buf {
                                    rocket_usart.write_byte(byte);
                                }
                            }
                    }
                }
    
                Err(e) => {}
            }
    
            if request_success {                
                pin_state.update(pin_set);

                
                let len_encoded = encode_into_slice(pin_state, 
                    &mut write_buf, 
                    standard()
                ).unwrap();
    
                match i2c_slave.respond(&write_buf) {
                    Ok(bytes_sent) => ufmt::uwriteln!(serial,
                                "{} bytes sent",
                                bytes_sent
                            ).unwrap(),
                            
                    Err(err) => uwriteln!(&mut serial, 
                        "Error: {:?}", 
                        err).unwrap(),
                }
            }
    
            write_buf.fill(0);

        }
    }
}
