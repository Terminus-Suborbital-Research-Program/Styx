
#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use core::{
    ptr::write_bytes,
    sync::atomic::{AtomicBool, Ordering},
};


use atmega_hal::{clock::MHz16, pac::{USART0, USART2}, port::{mode::{Floating, Input, Output}, PE0, PE1, PH0, PH1}};
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
use handler::*;
use panic_halt as _;
use ufmt::{uWrite, uwrite, uwriteln};

type CoreClock = atmega_hal::clock::MHz16;
type Delay = atmega_hal::delay::Delay<crate::CoreClock>;

mod i2c_slave;
mod handler;


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



#[avr_device::entry]
fn main() -> ! {
    let dp = atmega_hal::Peripherals::take().unwrap();
    let pins = atmega_hal::pins!(dp);

    let pin_set = [
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

    i2c_slave.init(false);

    // External Interrupt Control Register A modified to detect interrupts
    dp.EXINT.eicra.modify(|_, w| w.isc3().bits(0x03));  // Rising edge interrupt configured
    dp.EXINT.eimsk.modify(|_, w| w.int().bits(0x04));  // A specific pin is enabled for interrupt

    // Enable global interrupt
    unsafe { avr_device::interrupt::enable() };

    led.set_low();


    let mut pi_buf: [u8; 20] = [0u8; 20];

    let mut ejector_buf: Vec<u8, 30> = Vec::new();

    let mut write_buf: [u8; 20] = [0u8; 20];

    let pin_state = PinState::new(&pin_set);

    let mut message_handler = MessageHandler::new(
        &i2c_slave, 
        pin_state, 
        pin_set, 
        write_buf, 
        pi_buf);


    loop {
            if REQUEST.load(Ordering::SeqCst) {
                loop {
                    match ejector_usart.read() {
                        Ok(byte) => {
                            ejector_buf.push(byte);
                            uwriteln!(serial, "Byte read: {}", byte).ok();
                        }
        
                        Err(nb::Error::WouldBlock) => { 
                                // turn off interrupt flag
                                // Send raw encoded packet to rocket
                            for byte in &ejector_buf {
                                rocket_usart.write_byte(*byte);
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

            match i2c_slave.receive(&mut pi_buf) {
                    Ok(_) => {
                        uwriteln!(&mut serial, "Received packet ").ok();
                        message_handler.handle_packet(&mut serial, &mut rocket_usart);
                    }
                    Err(err) => {
                        uwriteln!(&mut serial, "Error: {:?}", err).ok();
                    }
            }
        }
}

#[avr_device::interrupt(atmega2560)]
fn INT2() {
    REQUEST.store(true, Ordering::SeqCst);
}
