#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

/*
 This is derived from Rahix' comment to
https://github.com/Rahix/avr-hal/issues/75
and then modernized to account for API drift since 2020
*/
use embedded_hal::delay::DelayNs;

use atmega_hal::port::mode::{Output,Input};
use atmega_hal::port::Pin;
//use atmega_hal::prelude::*;
use atmega_hal::usart::{Baudrate, Usart};
use atmega_hal::prelude::_unwrap_infallible_UnwrapInfallible;

use avr_device::atmega2560::tc1::tccr1b::CS1_A;
use avr_device::atmega2560::TC1;
//use embedded_hal::i2c::I2c;

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

fn delay_ms(ms: u16) {
    Delay::new().delay_ms(u32::from(ms))
}

#[allow(dead_code)]
fn delay_us(us: u32) {
    Delay::new().delay_us(us)
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

    
    let mut i2c = I2c::new(
        dp.TWI, 
        pins.pd1.into_pull_up_input(), //Check later but these should be right
        pins.pd0.into_pull_up_input(), // Also check if these are genuinely configured to pull-up or if external resistors are required
        50_000); //Also try with 100_000

    ufmt::uwriteln!(&mut serial, "Write direction:\r").unwrap();
    
    // Should find addreses on the bus attached to pd1 and pd0
    i2c.i2cdetect(&mut serial, atmega_hal::i2c::Direction::Write).unwrap();
    i2c.i2cdetect(&mut serial, atmega_hal::i2c::Direction::Read).unwrap();

    // ignore err for now
    //let _ = i2c.write(ADDRESS, &[1, 2, 3]);
    
    loop {}


}
