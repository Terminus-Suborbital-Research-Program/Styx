use bincode::{config::standard, error::DecodeError};
use core::{fmt::Write, task::Poll};
use defmt::{error, info, trace};
use embedded_hal::digital::StatefulOutputPin;
use embedded_hal_async::i2c::I2c;
use embedded_io::Read;
use fugit::ExtU64;
use futures::FutureExt as _;
use rtic::Mutex;
use rtic_monotonics::Monotonic;

use crate::{
    app::{incoming_packet_handler, *},
    communications::{
        hc12::BaudRate,
        link_layer::{LinkLayerPayload, LinkPacket},
    },
    device_constants::MotorI2cBus,
    Mono,
};

pub async fn heartbeat(ctx: heartbeat::Context<'_>) {
    loop {
        _ = ctx.local.led.toggle();

        Mono::delay(300_u64.millis()).await;
    }
}

pub fn uart_interrupt(mut ctx: uart_interrupt::Context<'_>) {
    ctx.shared.radio_link.lock(|radio| {
        radio.device.update().ok();
    });
}

use rp235x_pac::interrupt;
#[interrupt]
unsafe fn I2C0_IRQ() {
    use rp235x_hal::async_utils::AsyncPeripheral;
    MotorI2cBus::on_interrupt();
}

pub async fn motor_drivers(ctx: motor_drivers::Context<'_>) {
    info!("Motor driver task started!");

    // Motor driver i2c
    let motor_i2c = ctx.local.motor_i2c_bus;

    loop {
        // Gotta unmask the NVIC core, we're not using the RTIC scheduler for this
        unsafe {
            cortex_m::peripheral::NVIC::unpend(rp235x_hal::pac::Interrupt::I2C0_IRQ);
            cortex_m::peripheral::NVIC::unmask(rp235x_hal::pac::Interrupt::I2C0_IRQ);
        }

        // The speed is set through the DIGITAL_SPEED_CTRL field as a unsigned 14 bit value
        // This is bytes 16-30. Why they chose a 14 bit field is beyond me
        // This gives us a max speed of 32767
        let speed_percent = 0;
        let speed = ((speed_percent as u32) * 32767) / 100;
        let speed = speed & 0x3FFF; // Mask to 14 bits
        let speed = speed << 16; // Shift to the right place

        // let first_byte = ((speed >> 24) & 0xFF) as u8;
        // let second_byte = ((speed >> 16) & 0xFF) as u8;
        // let third_byte = ((speed >> 8) & 0xFF) as u8;
        // let fourth_byte = (speed & 0xFF) as u8;

        // BRRRRT! WRONG, ITS LSB FIRST
        let first_byte = (speed & 0xFF) as u8;
        let second_byte = ((speed >> 8) & 0xFF) as u8;
        let third_byte = ((speed >> 16) & 0xFF) as u8;
        let fourth_byte = ((speed >> 24) & 0xFF) as u8;

        // Register 0xec
        let bytes: [u8; 7] = [
            0b0001_0000,
            0b0000_0000u8,
            0xec,
            first_byte,
            second_byte,
            third_byte,
            fourth_byte,
        ];

        let mut cnt = 0;
        let timeout = core::future::poll_fn(|cx| {
            trace!("Waker called!");
            cx.waker().wake_by_ref();
            match cnt {
                10 => Poll::Ready(()),
                _ => {
                    cnt += 1;

                    Poll::Pending
                }
            }
        });

        // Fuse, timing out if we don't get a response
        info!("Writing Speed...");
        futures::select_biased! {
            r = motor_i2c.write(0x26u8, &bytes).fuse() => {
                info!("I2c write result: {:?}", r);
            }

            _ = timeout.fuse() => {
                info!("I2C Timeout!");
            }
        }

        Mono::delay(500_u64.millis()).await;
    }
}

pub async fn radio_flush(mut ctx: radio_flush::Context<'_>) {
    let mut on_board_baudrate: BaudRate = BaudRate::B9600;
    let bytes_to_flush = 16;

    loop {
        ctx.shared.radio_link.lock(|radio| {
            radio.device.flush(bytes_to_flush).ok();
            on_board_baudrate = radio.device.get_baudrate();
        });

        // Need to wait wait the in-air baudrate, or the on-board baudrate
        // whichever is slower

        let mut slower =
            core::cmp::min(on_board_baudrate.to_u32(), on_board_baudrate.to_in_air_bd());

        // slower is bps, so /1000 to get ms
        slower /= 1000;

        // Delay for that times the number of bytes flushed
        Mono::delay((slower as u64 * bytes_to_flush as u64).millis()).await;
    }
}

pub async fn incoming_packet_handler(mut ctx: incoming_packet_handler::Context<'_>) {
    loop {
        let buffer = ctx
            .shared
            .radio_link
            .lock(|radio| radio.device.clone_buffer());

        let decode: Result<(LinkPacket, usize), bincode::error::DecodeError> =
            bincode::decode_from_slice(&buffer, standard());

        match decode {
            Err(e) => match e {
                #[allow(unused_variables)]
                DecodeError::UnexpectedVariant {
                    type_name,
                    allowed,
                    found,
                } => {
                    // Clear the buffer
                    ctx.shared.radio_link.lock(|radio| radio.device.clear());
                }

                #[allow(unused_variables)]
                DecodeError::UnexpectedEnd { additional } => {
                    // Nothing to do here
                }

                // These decoding errors cause us to pop the front of the buffer to remove the character
                #[allow(unused_variables)]
                DecodeError::InvalidIntegerType { expected, found } => {
                    let mut buffer = [0u8; 1];
                    ctx.shared
                        .radio_link
                        .lock(|radio| radio.device.read(&mut buffer).ok());
                }

                _ => {
                    // Something else, log it. Bincode doen't impliment defmt::Debug unfortunately
                    error!("Decoding error: {:?}", defmt::Debug2Format(&e));
                }
            },

            Ok(packet_wrapper) => {
                let packet = packet_wrapper.0;
                let read = packet_wrapper.1;
                // Drop the read bytes
                ctx.shared
                    .radio_link
                    .lock(|radio| radio.device.drop_bytes(read));

                // Uncomment the below if you think you made a mistake in handling

                // let mut buffer_heapless_stirng: alloc::string::String =
                //     alloc::string::String::new();
                // write!(buffer_heapless_stirng, "{:#?}", packet).ok();
                // for char in buffer_heapless_stirng.chars() {
                //     print!(ctx, "{}", char);
                //     Mono::delay(1_u64.millis()).await;
                // }
                // println!(ctx, "\n");

                // Check the checksum, if it fails, the packet is bad, we should continue
                // and clear the buffer
                if !packet.verify_checksum() {
                    ctx.shared.radio_link.lock(|radio| radio.device.clear());
                    continue;
                }

                if let LinkLayerPayload::Payload(app_packet) = packet.payload {
                    match app_packet {
                        bin_packets::ApplicationPacket::Command(command) => match command {
                            _ => {}
                        },

                        _ => {
                            let mut buffer_heapless_stirng: alloc::string::String =
                                alloc::string::String::new();
                            write!(buffer_heapless_stirng, "{:#?}", packet).ok();
                        }
                    }
                }
            }
        }

        Mono::delay(10_u64.millis()).await;
    }
}

pub async fn sample_sensors(_ctx: sample_sensors::Context<'_>) {
    loop {
        Mono::delay(250_u64.millis()).await;
    }
}

pub async fn inertial_nav(_ctx: inertial_nav::Context<'_>){
    // TODO: Implement the inertial navigation functionality
}
