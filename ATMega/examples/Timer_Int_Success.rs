#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

/*
 This is derived from Rahix' comment to
https://github.com/Rahix/avr-hal/issues/75
and then modernized to account for API drift since 2020

*/

use atmega_hal::port::mode::Output;
use atmega_hal::port::Pin;
use atmega_hal::prelude::*;
use atmega_hal::usart::{Baudrate, Usart};
use avr_device::atmega2560::tc1::tccr1b::CS1_A;
use avr_device::atmega2560::TC1;
use core::mem;
use panic_halt as _;
use ufmt::{uWrite, uwriteln};

type CoreClock = atmega_hal::clock::MHz16;

struct InterruptState {
    blinker: Pin<Output>,
}

static mut INTERRUPT_STATE: mem::MaybeUninit<InterruptState> = mem::MaybeUninit::uninit();

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
    ufmt::uwriteln!(&mut serial, "Hello from Atmega!\r").unwrap_infallible();

    let led = pins.pb7.into_output();

    unsafe {
        // SAFETY: Interrupts are not enabled at this point so we can safely write the global
        // variable here.  A memory barrier afterwards ensures the compiler won't reorder this
        // after any operation that enables interrupts.
        INTERRUPT_STATE = mem::MaybeUninit::new(InterruptState {
            blinker: led.downgrade(),
        });
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }

    //

    let tmr1: TC1 = dp.TC1;

    rig_timer(&tmr1, &mut serial);

    // Enable interrupts globally, not a replacement for the specific interrupt enable
    unsafe {
        // SAFETY: Not inside a critical section and any non-atomic operations have been completed
        // at this point.
        avr_device::interrupt::enable();
    }

    ufmt::uwriteln!(
        &mut serial,
        "configured timer output compare register = {}",
        tmr1.ocr1a.read().bits()
    )
    .unwrap_infallible();

    loop {
        avr_device::asm::sleep()
        //ufmt::uwriteln!(&mut serial, "Tick \n").unwrap_infallible();
    }
}

pub const fn calc_overflow(clock_hz: u32, target_hz: u32, prescale: u32) -> u32 {
    /*
    https://github.com/Rahix/avr-hal/issues/75
    reversing the formula F = 16 MHz / (256 * (1 + 15624)) = 4 Hz
     */
    clock_hz / target_hz / prescale - 1
}

pub fn rig_timer<W: uWrite<Error = ::core::convert::Infallible>>(tmr1: &TC1, serial: &mut W) { //
    /*
     https://ww1.microchip.com/downloads/en/DeviceDoc/Atmel-7810-Automotive-Microcontrollers-ATmega328P_Datasheet.pdf
     section 15.11
    */
    use atmega_hal::clock::Clock;

    // Have to rig clock frequency manually
    const ATMEGA_CLOCK_FREQUENCY_HZ: u32 = CoreClock::FREQ;
    const CLOCK_SOURCE: CS1_A = CS1_A::PRESCALE_256;
    let clock_divisor: u32 = match CLOCK_SOURCE {
        CS1_A::DIRECT => 1,
        CS1_A::PRESCALE_8 => 8,
        CS1_A::PRESCALE_64 => 64,
        CS1_A::PRESCALE_256 => 256,
        CS1_A::PRESCALE_1024 => 1024,
        CS1_A::NO_CLOCK | CS1_A::EXT_FALLING | CS1_A::EXT_RISING => {
            uwriteln!(serial, "uhoh, code tried to set the clock source to something other than a static prescaler {}", CLOCK_SOURCE as usize)
                .unwrap_infallible();
            1
        }
    };

    let ticks = calc_overflow(ATMEGA_CLOCK_FREQUENCY_HZ, 1, clock_divisor) as u16;
    ufmt::uwriteln!(
        serial,
        "configuring timer output compare register = {}",
        ticks
    )
    .unwrap_infallible();

    tmr1.tccr1a.write(|w| w.wgm1().bits(0b00));
    tmr1.tccr1b.write(|w| {
        w.cs1()
            //.prescale_256()
            .variant(CLOCK_SOURCE)
            .wgm1()
            .bits(0b01)
    });
    tmr1.ocr1a.write(|w| w.bits(ticks));
    tmr1.timsk1.write(|w| w.ocie1a().set_bit()); //enable this specific interrupt
}

#[avr_device::interrupt(atmega2560)]
fn TIMER1_COMPA() {
    let state = unsafe {
        // SAFETY: We _know_ that interrupts will only be enabled after the LED global was
        // initialized so this ISR will never run when LED is uninitialized.
        &mut *INTERRUPT_STATE.as_mut_ptr()
    };

    state.blinker.toggle();
}