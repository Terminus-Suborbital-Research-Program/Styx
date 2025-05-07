
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

static 

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
    let dp = atmega_hal::Peripherals::take().ok();
    let pins = atmega_hal::pins!(dp);

    let mut serial = Usart::new(
        dp.USART0,
        pins.pe0,
        pins.pe1.into_output(),
        Baudrate::<crate::CoreClock>::new(57600),
    );

    // let mut rocket_usart = Usart::new(
    //     dp.USART2, 
    //     pins.ph0, 
    //     pins.ph1.into_output(), 
    //     Baudrate::<crate::CoreClock>::new(9600)
    // );

    let mut led = pins.pb7.into_output();

    let pin_set = &[
        pins.pa0.into_floating_input().downgrade(),
        pins.pa1.into_floating_input().downgrade(),
        pins.pa2.into_floating_input().downgrade(),
        pins.pa3.into_floating_input().downgrade(),
        pins.pa4.into_floating_input().downgrade(),
        pins.pa5.into_floating_input().downgrade(),
        pins.pa6.into_floating_input().downgrade(),
    ];

    let mut pin_state = PinState::new(pin_set);
    // Using external pullup resistors, so pins configured as floating inputWs
    let sda = pins.pd1.into_floating_input();
    let scl = pins.pd0.into_floating_input();
    
    let slave_address: u8 = 0x26;

    let mut i2c_slave: I2cSlave = I2cSlave::new(dp.TWI, slave_address, sda, scl, &TWI_INT_FLAG);

    // Enable global interrupt
    unsafe { avr_device::interrupt::enable() };

    // Disabling power reduction for TWI
    //dp.CPU.prr.write(|w| w.prtwi().clear_bit());

    // Value recieved from I2C Master
    let mut buf: [u8; 20];

    ufmt::uwriteln!(&mut serial, "Initialized with addr: 0x{:X}", slave_address).ok();

    led.set_low();


    // Check in and out of loop
    i2c_slave.init(false);
    let mut read_buf: [u8; 20] = [0u8; 20];

    loop {

        // RECEIVE
        // match i2c_slave.receive(&mut read_buf) {
        //     Ok(_) => {
        //         uwrite!(&mut serial, "Received: ").ok();

        //         read_buf.iter().for_each(|b| {
        //             uwrite!(&mut serial, "{} ", *b).ok();
        //         });
        //         uwrite!(&mut serial, "\n").ok();
        //     }
        //     Err(err) => {
        //         uwriteln!(&mut serial, "Error: {:?}", err).ok();
        //     }
        // };
        
        i2c_slave.receive(&mut read_buf).unwrap_or(());

        let mut request_success: bool = false;

        let request: Result<(
            ApplicationPacket, usize), bincode::error::DecodeError>  = decode_from_slice(&read_buf, standard());

        match request {
            Ok((app_packet, len)) => {
                match app_packet {
                    ApplicationPacket::Command(CommandPacket::Ping) => {
                            ufmt::uwriteln!(serial, "App Packet Ping found").ok();
                            request_success = true;
                        }
                        _ => {
                            // Send packets that are not a ping to rocket
                            for byte in read_buf {
                                rocket_usart.write_byte(byte);
                            }
                            pin_state.update(pin_set);
                        }
                }
            }

            Err(e) => {}
        }

        for byte in read_buf {
            rocket_usart.write_byte(byte);
        }
        
        pin_state.update(pin_set);

        if request_success {                
            pin_state.update(pin_set);
            let mut write_buf: [u8; 20] = [0u8; 20];

            match encode_into_slice(pin_state, &mut write_buf, standard()) {
                Ok(len_encoded) => { uwriteln!(serial, "{} bytes encoded", len_encoded).ok(); }
                
                Err(e) => { uwriteln!(serial, "encode error").ok(); }
            }

            match i2c_slave.respond(&write_buf) {
                Ok(bytes_sent) => ufmt::uwriteln!(serial,
                            "{} bytes sent",
                            bytes_sent
                        ).ok(),
                        
                Err(err) => uwriteln!(&mut serial, 
                    "Error: {:?}", 
                    err).ok(),
            }
        }

        read_buf.fill(0);
    }
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