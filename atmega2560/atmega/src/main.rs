#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use core::sync::atomic::{AtomicBool, Ordering};

use atmega_hal::{
    clock::MHz16,
    usart::{Baudrate, Usart},
};

use common::indicators::IndicatorBuilder;
use i2c_slave::*;
use panic_halt as _;
use ufmt::{uwriteln, uwrite};

type CoreClock = MHz16;

mod i2c_slave;
static TWI_INT_FLAG: AtomicBool = AtomicBool::new(false);

// // I2C interrupt handler
#[avr_device::interrupt(atmega2560)]
fn TWI() {
    avr_device::interrupt::free(|_| {
        TWI_INT_FLAG.store(true, Ordering::SeqCst);
    });
}

use arduino_hal as atmega_hal;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = atmega_hal::Peripherals::take().unwrap();
    let pins = atmega_hal::pins!(dp);

    let mut serial = Usart::new(
        dp.USART0,
        pins.d0,
        pins.d1.into_output(),
        Baudrate::<crate::CoreClock>::new(57600),
    );

    let mut led = pins.d13.into_output();

    let mut battery_latch = pins.d29.into_output();

    let gse1 = pins.d22.into_floating_input().downgrade();
    let gse2 = pins.d23.into_floating_input().downgrade();
    let te_ra = pins.d24.into_floating_input().downgrade();
    let te_rb = pins.d25.into_floating_input().downgrade();
    let te_1 = pins.d26.into_floating_input().downgrade();
    let te_2 = pins.d27.into_floating_input().downgrade();
    let te_3 = pins.d28.into_floating_input().downgrade();

    // Using external pullup resistors, so pins configured as floating inputWs
    let sda = pins.d20.into_floating_input();
    let scl = pins.d21.into_floating_input();

    let slave_address: u8 = 0x26;

    let mut i2c_slave: I2cSlave = I2cSlave::new(dp.TWI, slave_address, sda, scl, &TWI_INT_FLAG);

    // Enable global interrupt
    unsafe { avr_device::interrupt::enable() };

    uwriteln!(&mut serial, "Initialized with addr: 0x{:X}", slave_address).ok();
    i2c_slave.init(false);

    led.set_low();

    // Check in and out of loop

    let mut write_buf: [u8; 1] = [0u8; 1];
    let mut read_buf: [u8; 2] = [0u8; 2];

    loop {
        let pins = IndicatorBuilder::new()
            .gse1(gse1.is_high())
            .gse2(gse2.is_high())
            .te_ra(te_ra.is_high())
            .te_rb(te_rb.is_high())
            .te1(te_1.is_high())
            .te2(te_2.is_high())
            .te3(te_3.is_high())
            .build();

        write_buf[0] = pins.into();

        // match i2c_slave.respond(&write_buf) {
        //     Ok(bytes_sent) => {
        //         uwriteln!(serial, "{} bytes sent", bytes_sent).ok();
        //     }

        //     Err(err) => {
        //         uwriteln!(serial, "Response Error: {:?}", err).ok();
        //     }
        // }

        match i2c_slave.receive(&mut read_buf) {
            Ok(_) => {
                uwriteln!(serial, "Successful read").ok();
                if read_buf[0] == 1 {
                    battery_latch.set_high();
                } else {
                    battery_latch.set_low();
                }
            }
            Err(err) => {
                uwriteln!(serial, "Error: {:?}", err).ok();
            }
        }

        // match i2c_slave.transact(&mut read_buf, &mut write_buf) {
        //     Ok(()) => {
        //         uwriteln!(serial, "Worked").ok();
        //         match read_buf[1] {
        //             0 => {}
        //             1 => {}
        //             2 => {}
        //         }
        //     }

        //     Err(err) => {
        //         uwriteln!(serial, "Response Error: {:?}", err).ok();
        //     }
            
        // }

        // for b in read_buf {
        //     uwrite!(serial, "{}", b).ok();
        // }
        // uwriteln!(serial, "").ok();

        read_buf.fill(0);
    }
}
