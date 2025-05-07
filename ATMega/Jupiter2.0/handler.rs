use atmega_hal::{clock::MHz16, pac::{USART0, USART2}, port::{mode::{Floating, Input, Output}, PE0, PE1, PH0, PH1}};
use atmega_hal::port::{self, Pin, Dynamic};
use atmega_hal::usart::{Baudrate, Usart};
use atmega_hal::prelude::_embedded_hal_serial_Read;

use heapless::Vec;
use bin_packets::{data::PinState, ApplicationPacket, CommandPacket};
use bincode::{
    config::standard, decode_from_slice, encode_into_slice, error::{DecodeError, EncodeError}
};
use ufmt::{uWrite, uwrite, uwriteln};
use crate::i2c_slave::*;
type PinArray = [Pin<Input<Floating>, Dynamic>; 7];

pub trait Read {
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

pub struct MessageHandler<'a, const BUF_SIZE: usize> {
    i2c_slave: &'a I2cSlave<'a>,
    pin_state:  PinState,
    pin_set: PinArray,
    write_buf: [u8; BUF_SIZE],
    pi_buf: [u8; BUF_SIZE],
}

impl <'a,const BUF_SIZE: usize> MessageHandler <'a,BUF_SIZE> { //, const BUF_SIZE: usize
    pub fn new(
        i2c_slave: &'a I2cSlave<'a>,
        pin_state:  PinState,
        pin_set: PinArray,
        write_buf: [u8; BUF_SIZE],
        pi_buf: [u8; BUF_SIZE], // rocket_usart: &'a mut Usart<USART2, Pin<Input, PH0>, Pin<Output, PH1>, MHz16>,
    ) -> Self {
        MessageHandler {
            i2c_slave,
            pin_state,
            pin_set,
            write_buf,
            pi_buf,
        }
    }

    pub fn handle_packet(&mut self, 
        serial: &mut Usart<USART0, Pin<Input, PE0>, Pin<Output, PE1>, MHz16>,
        rocket_usart: &mut Usart<USART2, Pin<Input, PH0>, Pin<Output, PH1>, MHz16>){

            let packet: Result<(ApplicationPacket, usize),
                                bincode::error::DecodeError> = decode_from_slice(&self.pi_buf, standard());

            match packet {
                Ok((app_packet, len)) => {
                    match app_packet {

                        // Ping, send pin_state
                        ApplicationPacket::Command(CommandPacket::Ping) => {
                                uwriteln!(serial, "App Packet Ping found").ok();
                                self.send_pinstate(serial);
                            }

                             // Send packets that are not a ping to rocket
                            _ => {
                                uwriteln!(serial,"Non_Ping Packet Sent to Rocket").ok();
                                self.rocket_write(rocket_usart);
                            }
                    }
                }
                
                // Decode error, send bytes to ground for later examination
                Err(e) => {
                    uwriteln!(serial,"Decoding Error").ok();
                    self.rocket_write(rocket_usart);
                }   
                
            }
            
    }
        
    pub fn rocket_write(&self, rocket_usart: &mut Usart<USART2, Pin<Input, PH0>, Pin<Output, PH1>, MHz16>) {
        for byte in &self.pi_buf {
            rocket_usart.write_byte(*byte);
        }
    }

    pub fn send_pinstate(&mut self, serial: &mut Usart<USART0, Pin<Input, PE0>, Pin<Output, PE1>, MHz16>) {
        self.pin_state.update(&self.pin_set);
        // Match this
        match encode_into_slice(self.pin_state, &mut self.write_buf, standard()) {
            Ok(len_encoded) => { uwriteln!(serial, "{} bytes encoded", len_encoded).ok(); }
            
            Err(e) => { uwriteln!(serial, "encode error").ok(); }
        }

        match self.i2c_slave.respond(&self.write_buf) {
            Ok(bytes_sent) => {
               uwriteln!(serial, "{} bytes sent", bytes_sent).ok();
            }
                    
            Err(err) => {
                uwriteln!(serial, "Error: {:?}", err).ok();
            }
        }
        self.write_buf.fill(0);
    }
}