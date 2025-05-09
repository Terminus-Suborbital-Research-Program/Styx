#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]



use atmega_hal::Pins;
/*
 This is derived from Rahix' comment to
https://github.com/Rahix/avr-hal/issues/75
and then modernized to account for API drift since 2020
*/
use embedded_hal::delay::DelayNs;

use bincode::{
    config::standard,
    enc::write::Writer,
    encode_into_slice,
    decode_from_slice,
    error::{DecodeError, EncodeError},
    Decode, Encode,
};



use atmega_hal::port::mode::{Floating, Input, Output};
use atmega_hal::port::{self, Pin};
//use atmega_hal::prelude::*;
use atmega_hal::usart::{Baudrate, Usart};
use atmega_hal::prelude::_unwrap_infallible_UnwrapInfallible;

use avr_device::atmega2560::tc1::tccr1b::CS1_A;
use avr_device::atmega2560::TC1;

use core::sync::atomic::{AtomicBool, Ordering};
// This requires disabling default features in Cargo.toml
use either::*;


use core::mem;
use panic_halt as _;
use ufmt::{uWrite, uwriteln};

type CoreClock = atmega_hal::clock::MHz16;

type Delay = atmega_hal::delay::Delay<crate::CoreClock>;
use embedded_hal::i2c::I2c as _;

type I2c = atmega_hal::i2c::I2c<crate::CoreClock>;

use core::panic::PanicInfo;


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
    let mut gse_1 = pins.pa0.into_floating_input(); // I think this should be pull down, but I'll talk wil Lucas abt it later
    let mut gse_2 = pins.pa1.into_floating_input(); // Will require external pulldown
    let mut te_ra = pins.pa2.into_floating_input(); 
    let mut te_rb = pins.pa3.into_floating_input();

    let mut te_1 = pins.pa4.into_floating_input();
    let mut te_2 = pins.pa5.into_floating_input();
    let mut te_3 = pins.pa6.into_floating_input();

    let mut serial = Usart::new(
        dp.USART0,
        pins.pe0,
        pins.pe1.into_output(),
        Baudrate::<crate::CoreClock>::new(57600),
    );
    
    let example = LineStatus {
        lines: 0x03,
        time: 0x05,
    };

    let mut slice = [0u8; 300];
 
    let len_encoded = encode_into_slice(example, &mut slice, standard()).unwrap(); // Returns size of encoded slice??
    
    let shit: Result<(LineStatus, usize), bincode::error::DecodeError>  = decode_from_slice(&slice, standard());

    match shit {

        Ok(decode_touple) => {

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
