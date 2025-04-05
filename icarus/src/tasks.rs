use bincode::{config::standard, error::DecodeError};
use core::fmt::Write;
use embedded_hal::digital::StatefulOutputPin;
use embedded_io::Read;
use fugit::ExtU64;
use icarus::{print, println};
use rtic::Mutex;
use rtic_monotonics::Monotonic;

use crate::{
    app::{incoming_packet_handler, *},
    communications::{
        hc12::BaudRate,
        link_layer::{LinkLayerPayload, LinkPacket},
    },
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
                    let mut buffer = alloc::string::String::new();
                    for c in buffer.chars() {
                        Mono::delay(1_u64.millis()).await;
                    }
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
                            for char in buffer_heapless_stirng.chars() {
                                Mono::delay(1_u64.millis()).await;
                            }
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
        // ctx.shared.software_delay.lock(|mut delay|{
        //     ctx.shared.env_sensor.lock(|environment|{
        //         let measurements = environment.measure(&mut delay).unwrap();
        //         println!(ctx, "Measurements: {}", measurements.temperature);
        //     });
        // });
        Mono::delay(250_u64.millis()).await;
    }
    // let pressure_result = environment.read_pressure();
    // match pressure_result{
    //     Ok(atomic_response) =>{
    //         match atomic_response{
    //             Some(pressure_value)=>{
    //                 println!(ctx, "Pressure: {}", pressure_value);
    //             }
    //             None =>{
    //                 println!(ctx, "No Pressure Result");
    //             }
    //         }
    //     }
    //     Err(atomic_response)=>{
    //         match atomic_response{
    //             embedded_hal_bus::i2c::AtomicError::Busy=>{
    //                 println!(ctx, "Busy...");
    //             }
    //             embedded_hal_bus::i2c::AtomicError::Other(other_value) =>{
    //                 match other_value{
    //                     rp235x_hal::i2c::Error::Abort(_)=>{
    //                         println!(ctx, "Aborted");
    //                     }
    //                     rp235x_hal::i2c::Error::InvalidReadBufferLength=>{
    //                         println!(ctx,"Invalid Read Buffer Length");
    //                     }
    //                     rp235x_hal::i2c::Error::InvalidWriteBufferLength=>{
    //                         println!(ctx,"Invalid Write Buffer Length");
    //                     }
    //                     rp235x_hal::i2c::Error::AddressOutOfRange(_)=>{
    //                         println!(ctx,"Address Out of Range");

    //                     }
    //                     rp235x_hal::i2c::Error::AddressReserved(_)=>{
    //                         println!(ctx,"Address Reserved");
    //                     }
    //                     _=>{
    //                         println!(ctx, "Something Happendededed");
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }
    // let temperature = environment.read_temperature().unwrap().unwrap();
    // let humidity = environment.read_humidity().unwrap().unwrap();

    // println!(ctx, "Pressure: {}", pressure);
    // println!(ctx, "Temperature: {}", pressure);
    // println!(ctx, "Humidity: {}", pressure);
}


pub async fn inertial_nav(_ctx: inertial_nav::Context<'_>){
    
}