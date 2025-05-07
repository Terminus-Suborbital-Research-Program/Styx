#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
use embedded_hal::delay::DelayNs;

use atmega_hal::port::mode::Output;
use atmega_hal::port::Pin;
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

static REVERSED: AtomicBool = AtomicBool::new(false);

fn delay_ms(ms: u16) {
    Delay::new().delay_ms(u32::from(ms))
}

#[allow(dead_code)]
fn delay_us(us: u32) {
    Delay::new().delay_us(us)
}

fn is_reversed() -> bool {
    REVERSED.load(Ordering::SeqCst)
}

fn blink_for_range<W: uWrite<Error = ::core::convert::Infallible>>(range: impl Iterator<Item = u16>, leds: &mut [Pin<Output>], serial: &mut W) {
    
    range.map(|i| i * 100).for_each(|ms| {
        let iter = if is_reversed() {
            Left(leds.iter_mut().rev())
        } else {
            Right(leds.iter_mut())
        };
        iter.for_each(|led| {
            let current_state = is_reversed();
            ufmt::uwriteln!(serial, "State: {} \n",  current_state).unwrap_infallible();
            led.toggle();
            delay_ms(ms as u16);
        })
    });
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

    // We are configuring the external interrupt for the pin corresponding to external interrupt 0
    dp.EXINT.eicra.modify(|_, w| w.isc0().bits(0x02));
    dp.EXINT.eimsk.modify(|_, w| w.int().bits(0x01)); 
    // set the starting bit of the register,which corresponds to enabling external interrupts on INT0
    // EIMSK must be modified to allow external interrupts on this pin

    // PCINT0 is PB0

    let mut leds: [Pin<Output>; 3] = [
        pins.pb1.into_output().downgrade(),
        pins.pb2.into_output().downgrade(),
        pins.pb3.into_output().downgrade(),
    ];



    unsafe {
        avr_device::interrupt::enable();
    }

    loop {
        
        blink_for_range(0..10, &mut leds, &mut serial);
        blink_for_range((0..10).rev(), &mut leds, &mut serial);
        //ufmt::uwriteln!(&mut serial, "State:  \n").unwrap_infallible();
    }
}

#[avr_device::interrupt(atmega2560)]
fn INT0() {
    let current = REVERSED.load(Ordering::SeqCst);
    REVERSED.store(!current, Ordering::SeqCst);
}