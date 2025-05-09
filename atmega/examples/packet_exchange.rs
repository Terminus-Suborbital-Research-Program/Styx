#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]


//use core::error::Error;

use atmega_hal::{port::{Dynamic, PA0}, Pins};

use embedded_hal::{delay::DelayNs, digital::InputPin};

use bincode::{
    config::standard, decode_from_slice, encode_into_slice, error::{DecodeError, EncodeError}
};

use core::sync::atomic::{AtomicBool, Ordering};
use heapless::Vec;

use atmega_hal::port::mode::{Floating, Input, Output};
use atmega_hal::port::{self, Pin};
use atmega_hal::usart::{Baudrate, Usart};
use atmega_hal::prelude::_embedded_hal_serial_Read;

use panic_halt as _;

use bin_packets::{data::PinState, ApplicationPacket, CommandPacket};

type CoreClock = atmega_hal::clock::MHz16;

type Delay = atmega_hal::delay::Delay<crate::CoreClock>;

type I2c = atmega_hal::i2c::I2c<crate::CoreClock>;

static REQUEST: AtomicBool = AtomicBool::new(false);
trait Read {
    fn new(pin_set: &[Pin<Input<Floating>, Dynamic>; 7]) -> Self ;
}

impl Read for PinState {
    fn new(pin_set: &[Pin<Input<Floating>, Dynamic>; 7]) -> Self {
        //.Vec<bool, 7>
        let state: Vec<bool, 7> = pin_set.iter().map(|pin| pin.is_high()).collect();
        //let mut vec: Vec<bool, 7> = Vec::new();
        //let state = pin_set.iter().map(|pin| pin.is_high()).collect_into(vec);
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

#[avr_device::entry]
fn main() -> ! {

    let dp = atmega_hal::Peripherals::take().unwrap();
    let pins = atmega_hal::pins!(dp);
    //: [Pin] 
    
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

    let mut uart1 = Usart::new(
        dp.USART1, 
        pins.pd2, 
        pins.pd3.into_output(), 
        Baudrate::<crate::CoreClock>::new(9600)
    );

    // External Interrupt Control Register A modified to detect 
    dp.EXINT.eicra.modify(|_, w| w.isc3().bits(0x03)); // 1100_0000 For ISC31 and ISC30 rising edge trigger configuration
    dp.EXINT.eimsk.modify(|_, w| w.int().bits(0x04)); // 0000_0100 For Int3 enable. This also clears the entire register
                                                              // but this is fine when just using interrupt 3

    unsafe {
        avr_device::interrupt::enable();
    }

    let mut del = Delay::new();
    let mut read_buf: Vec<u8, 500> = Vec::new();
    loop {

        // uart1.listen(event); This supposedly enables interrupt events but not sure how this method works
        //                      will look into later. Using normal external interrupt methods for now
        ////////////////////////////////////
        
        Delay::delay_ms(&mut del, 1000);
        
        //////////////////////////
        // Check the flag from interrupt
        if REQUEST.load(Ordering::SeqCst) {
            ufmt::uwriteln!(
                serial,
                "Interrupt triggered?"
            ).unwrap();
            loop {
                match uart1.read() {
                    Ok(byte) => {
                        read_buf.push(byte);
                        ufmt::uwriteln!(
                            serial,
                            "Byte read: {}",
                            byte
                        ).unwrap();
                    }
    
                    Err(nb::Error::WouldBlock) => { 
                        break;
                    }
                    
                    Err(nb::Error::Other(_e)) => { 
                        continue; //Throw erroneous bytes and iterate through every byte in hopes we find a good packet
                    }
                }
            } 


            // The result must be a ping
            let request: Result<(
                ApplicationPacket, usize), bincode::error::DecodeError>  = decode_from_slice(read_buf.as_slice(), standard());

            let mut request_success: bool = false;

            match request {
                Ok((app_packet, len)) => {
                    match app_packet {
                        ApplicationPacket::Command(CommandPacket::Ping) => {

                                ufmt::uwriteln!(
                                    serial,
                                    "App Packet Ping found",
                                ).unwrap();
                                request_success = true;
                            }

                            _ => {ufmt::uwriteln!(
                                serial,
                                "App Packet found, but no Ping recieved",
                            ).unwrap();
                        }
                    }

                }

                Err(e) => {
                        ufmt::uwriteln!(
                        serial,
                        "Decoding Error"
                    ).unwrap();
                }
            }

            if request_success {                
                let pin_state = PinState::new(pin_set);
                let mut slice: [u8; 20] = [0u8; 20]; // This probably doesn't have to be this long
                let len_encoded = encode_into_slice(pin_state, &mut slice, standard()).unwrap();
                // Send current pinstate to pi
                for byte in slice {
                    uart1.write_byte(byte);
                }
            }

            read_buf.clear();
                    
            // turn off interrupt flag
            REQUEST.store(false, Ordering::SeqCst);
        }
    }

}


#[avr_device::interrupt(atmega2560)]
fn INT2() {
    REQUEST.store(true, Ordering::SeqCst);
}