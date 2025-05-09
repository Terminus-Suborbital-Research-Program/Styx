#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]


use atmega_hal::{port::PA0, Pins};
use either::IntoEither;
/*
 This is derived from Rahix' comment to
https://github.com/Rahix/avr-hal/issues/75
and then modernized to account for API drift since 2020
*/
use embedded_hal::delay::DelayNs;

use bincode::{
    config::standard, de, decode_from_slice, enc::write::Writer, encode_into_slice, error::{DecodeError, EncodeError}, Decode, Encode
};



use atmega_hal::port::mode::{Floating, Input, Output};
use atmega_hal::port::{self, Pin};
use atmega_hal::prelude::*;

use atmega_hal::usart::{Baudrate, Usart};

// This requires disabling default features in Cargo.toml
use panic_halt as _;

type CoreClock = atmega_hal::clock::MHz16;

type Delay = atmega_hal::delay::Delay<crate::CoreClock>;

type I2c = atmega_hal::i2c::I2c<crate::CoreClock>;


//#[cfg(feature = "derive")]
#[derive(Debug, Clone, Copy, Encode, Decode)]
struct LineStatus {
    lines: u8,
    time: u16,
}


#[avr_device::entry]
fn main() -> ! {

    const ADDRESS: u8 = 0x69;
    let dp = atmega_hal::Peripherals::take().unwrap();
    let pins = atmega_hal::pins!(dp);

    
    let mut serial = Usart::new(
        dp.USART0,
        pins.pe0,
        pins.pe1.into_output(),
        Baudrate::<crate::CoreClock>::new(57600),
    );

    let mut uart1 = Usart::new(
        dp.USART1, 
        pins.pd2, 
        pins.pd3.into_output(), 
        Baudrate::<crate::CoreClock>::new(9600)
    );
    loop {
        ufmt::uwriteln!(
            uart1,
            "Hello",
        );

        let b = nb::block!(uart1.read()).unwrap();

        ufmt::uwriteln!(
            serial,
            "{}",
            b as char,
        );
    }
    
    
    let example = LineStatus {
        lines: 0x03,
        time: 0x05,
    };

    let mut slice = [0u8; 300];
 
    let len_encoded = encode_into_slice(example, &mut slice, standard()).unwrap(); // Returns size of encoded slice??
    
    let shit: Result<(LineStatus, usize), bincode::error::DecodeError>  = decode_from_slice(&slice, standard());

    match shit {

        Ok(decode_touple ) => {

            let line_status = decode_touple.0;
            let len = decode_touple.1;

            ufmt::uwriteln!(
                serial,
                "Shit = {} and {}, Len = {}",
                line_status.lines,
                line_status.time,
                len
            );
        }

        Err(e) => todo!()
    }
    
    loop {}
}
