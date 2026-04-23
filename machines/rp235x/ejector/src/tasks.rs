#![warn(missing_docs, clippy::unwrap_used)]

//! RTIC Task defintions for the Ejector

use crate::device_constants::RGBStatus;
use crate::sd_card::EJECTOR_GAURD_FILENAME;
use crate::{app::*, device_constants::SAMPLE_COUNT, sd_card, Mono};
use bin_packets::{
    commands::CommandPacket,
    devices::DeviceIdentifier,
    packets::{status::Status, ApplicationPacket},
    rgbstatus::RGBOptions,
};
use bincode::{config::standard, decode_from_slice, encode_into_slice, error::DecodeError};
use defmt::{debug, error, info, warn};
use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};
use embedded_io::{Read, ReadReady, Write};
use fugit::ExtU64;
use heapless::{deque::DequeInner, vec::ViewVecStorage, Deque, Vec};
use rtic::Mutex;
use rtic_monotonics::Monotonic;
use tinyframe::frame::Frame;
// use ws2812_rs::GlowColor;
// use rtic_sync::portable_atomic::{AtomicBool, Ordering};
use bin_packets::phases::EjectorPhase;
use rtic_sync::signal::Signal;
//use cortex_m::interrupt;
// use ws2812_rs::{Color, AsyncGlowColor};
// use ws2812_rs::{Color, GlowColor};
use ws2812_pio::Ws2812Direct;
use smart_leds::{SmartLedsWrite, RGB8};



#[cfg(not(feature = "fast-startup"))]
const JUPITER_BOOT_LOCKOUT_TIME_SECONDS: u64 = 180;
/// Constant to prevent ejector from interfering with JUPITER's u-boot sequence
#[cfg(feature = "fast-startup")]
const JUPITER_BOOT_LOCKOUT_TIME_SECONDS: u64 = 10;

const SHUTDOWN_TIME_CAMERAS: u64 = 210;

/// Task for sending heartbeat packets to JUPITER and toggling the onboard LED
pub async fn heartbeat(mut ctx: heartbeat::Context<'_>) {
    // let onboard_led = ctx.local.onboard_led;

    let mut sequence_number = 0;

    // Still blink, but toggle as it is done
    loop {
        // onboard_led.toggle().unwrap();
        // info!("Alive?");
        if Mono::now().duration_since_epoch().to_secs() > JUPITER_BOOT_LOCKOUT_TIME_SECONDS {
            let status = Status::new(DeviceIdentifier::Ejector, now_timestamp(), sequence_number);

            ctx.shared
                .downlink_packets
                .lock(|q| q.push_back(status.into()).ok());

            sequence_number = sequence_number.wrapping_add(1);
        }

        Mono::delay(300_u64.millis()).await;
    }
}

/// Task for sending downlink packets to JUPITER
pub async fn downlink_jupiter(mut ctx: downlink_jupiter::Context<'_>) {
    let mut enc_buf = [0u8; SCRATCH];
    let config = standard();
    loop {
        let packet = ctx
            .shared
            .downlink_packets
            .lock(|packets| packets.pop_front());

        if let Some(p) = packet {
            if let Ok(sz) = encode_into_slice(p, &mut enc_buf, config) {
                let _ = ctx.local.downlink.write_all(&enc_buf[..sz]);
            }
        } else {
            Mono::delay(50_u64.millis()).await;
        }
    }
}

const SCRATCH: usize = 512;

/// Task for camera sequencing
pub async fn camera_sequencer(mut ctx: camera_sequencer::Context<'_>) {
    // T+70, drive the cameras high
    Mono::delay(250.secs()).await;
    info!("Activating cameras!");
    ctx.local.camera_mosfet.set_high().ok();
    Mono::delay(SHUTDOWN_TIME_CAMERAS.secs()).await;
    info!("Shutting down cameras!");
    ctx.local.camera_mosfet.set_low().ok();
}

/// Task that manages the Ejector sequencing
///
/// NOTE: When the RBF pin is inserted, this task will idle and block ejection until the pin is removed.
pub async fn ejector_sequencer(mut ctx: ejector_sequencer::Context<'_>) {
    // while !ctx.shared.ejection_enabled.lock(|enabled| *enabled) {
    //     debug!("Ejector sequencer idling while RBF pin is inserted");
    //     Mono::delay(100_u64.millis()).await;
    // }

    let servo = ctx.local.ejector_servo;
    let e_magnet = ctx.local.ejecctor_magnet;

    // Latch ejector servos closed
    Mono::delay(1000_u64.millis()).await;
    servo.enable();
    // servo.hold();

// servo.servo.set_angle(150);
    info!("Ejecting!");

    // servo.eject();
    // servo.servo.set_angle(200);


    // s
    

    info!("Here");
    for i in (14..23) {
        info!("i {}!", i);

        let angle = (i * 10) as u16;
        info!("Set {}!", angle);

        servo.servo.set_angle(angle);
        info!("Set {}!", angle);
        Mono::delay(50_u64.millis()).await;
    }

    // info!("Here");
    // for i in (14..23).rev() {
    //     info!("i {}!", i);

    //     let angle = (i * 10) as u16;
    //     info!("Set {}!", angle);

    //     servo.servo.set_angle(angle);
    //     info!("Set {}!", angle);
    //     Mono::delay(100_u64.millis()).await;
    // }

    // for i in (5..14).rev() {
    //     info!("i {}!", i);

    //     let angle = (i * 10) as u16;
    //     info!("Set {}!", angle);

    //     servo.servo.set_angle(angle);
    //     info!("Set {}!", angle);
    //     Mono::delay(50_u64.millis()).await;
    // }

    // for i in (18..9) {
    //     let angle = (i * 10) as u16;
    //     servo.servo.set_angle(angle);
    //     info!("Set {}!", angle);
    //     Mono::delay(1000_u64.millis()).await;
    // }

    // servo.servo.set_angle();



    // Turn on the magnet
    e_magnet.enable();

    // let ejection_pin = ctx.local.ejection_pin;

    // Lockout for one minute to let JUPITER boot up
    // warn!("Idling sequencer");
    // Mono::delay(JUPITER_BOOT_LOCKOUT_TIME_SECONDS.secs()).await;
    // ctx.local.arming_led.set_low().ok();
    // info!("Sequencer unlocked, waiting for ejection signal");

    // ctx.local.ejection_trigger_rx.wait().await;
    // Right now we don't have a pin read from jupiter, although this may be re-added later
    // Wait until ejection pin from JUPITER reads high
    // while !ejection_pin.is_high().unwrap_or(false) {
    //     debug!("Ejector idling while waiting for ejection signal");
    //     Mono::delay(100_u64.millis()).await;
    // }

    // info!("Ejection signal high!");

    // Eject, wait 5 seconds, then retract
    e_magnet.polarity_switch();
    // loop {}
    // servo.eject();
    // servo.hold();
    info!("Hold!");

    servo.hold();


    // Give three seconds to retract, then disable to save power
    Mono::delay(7000_u64.millis()).await;
    e_magnet.polarity_switch();
    // servo.disable();
    e_magnet.disable();
    info!("Ejector disabled, servo and magnet disabled. Ejector sequencing complete.");
}

/// Task to measure the temperature for the thermal dissipation layer experiment
///
/// Timing: Every second
pub async fn poll_temperature(mut ctx: poll_temperature::Context<'_>) {
    let sensor = &mut ctx.local.thermocouple;

    info!("Mcp start");

    loop {
        match sensor.read_hot_junction() {
            Ok(temp) => info!("Thermocouple Temperature: {} C", temp),
            Err(_) => warn!("Failed to read MCP9600 thermocouple"),
        }

        Mono::delay(1000_u64.millis()).await;
    }
}

/// Task to poll the RBF pin and block ejection if it is inserted
///
/// Timing: Every 100 ms
pub async fn poll_rbf(mut ctx: poll_rbf::Context<'_>) {
    // loop {
    //     if ctx
    //         .local
    //         .rbf_pin
    //         .is_low()
    //         .expect("Failed to read the RBF pin state")
    //     {
    //         info!("RBF pin is low, blocking ejection code...");
    //         ctx.shared.ejection_enabled.lock(|blocked| *blocked = false);
    //     } else {
    //         info!("RBF pin is high, ejection code enabled.");
    //         ctx.shared.ejection_enabled.lock(|blocked| *blocked = true);
    //     }
    // }
}

pub async fn write_sd_card(mut ctx: write_sd_card::Context<'_>) {
    ctx.shared.sd_card.lock(|sd_card| {
        let file_data =
            b"GLORY BE TO RUST!\nGLORY BE TO RUST!\nGLORY BE TO RUST!\nGLORY BE TO RUST!\n";
        info!("Berofe Writting!");
        sd_card.write_data(EJECTOR_GAURD_FILENAME, file_data);
        info!("After Writting!");
    });
}

pub async fn rx_from_jupiter(mut ctx: rx_from_jupiter::Context<'_>) {
    let jupiter_rx = ctx.local.status_link;
    let config = standard();

    let mut rx_buf = [0u8; SCRATCH];
    let mut idx = 0;

    loop {
        let mut data_received = false;

        // Read all available bytes into the unused end
        while jupiter_rx.read_ready().expect("RX Uart fault") {
            // Prevent buffer overflow if garbage data fills the array
            if idx >= SCRATCH {
                error!("RX buffer overflow, dropping oldest byte");
                rx_buf.copy_within(1..idx, 0);
                idx -= 1;
            }

            match jupiter_rx.read(&mut rx_buf[idx..]) {
                Ok(bytes_read) if bytes_read > 0 => {
                    idx += bytes_read;
                    data_received = true;
                }
                Ok(_) => break, // 0 bytes read
                Err(_) => {
                    error!("Error reading bytes from uart rx");
                    break;
                }
            }
        }

        // Decode if bytes read
        while idx > 0 {
            match decode_from_slice::<ApplicationPacket, _>(&rx_buf[..idx], config) {
                Ok((
                    ApplicationPacket::Command(CommandPacket::ColorSet(status_options)),
                    bytes_used,
                )) => {
                    ctx.shared.status_config.lock(|status_config| {
                        status_config.update_from_options(status_options);
                    });

                    let remaining = idx - bytes_used;
                    if remaining > 0 {
                        rx_buf.copy_within(bytes_used..idx, 0);
                    }
                    idx = remaining;
                }

                // This would be way better with just a pin toggle
                Ok((
                    ApplicationPacket::Command(CommandPacket::EjectorPhaseSet(
                        EjectorPhase::Ejection,
                    )),
                    bytes_used,
                )) => {
                    ctx.local.ejection_trigger_tx.write(());

                    let remaining = idx - bytes_used;
                    if remaining > 0 {
                        rx_buf.copy_within(bytes_used..idx, 0);
                    }
                    idx = remaining;
                }

                // Successfully decoded a packet, but it's not a ColorSet command
                Ok((_, bytes_used)) => {
                    let remaining = idx - bytes_used;
                    if remaining > 0 {
                        rx_buf.copy_within(bytes_used..idx, 0);
                    }
                    idx = remaining;
                }

                // Incomplete packet: wait for more bytes on the next loop
                Err(bincode::error::DecodeError::UnexpectedEnd { .. }) => {
                    break;
                }

                // Corrupt data, so drop the oldest byte and slide the window to resync
                Err(_) => {
                    rx_buf.copy_within(1..idx, 0);
                    idx -= 1;
                }
            }
        }

        if !data_received {
            Mono::delay(10_u64.millis()).await;
        }
    }
}

// pub async fn set_rgb_status(mut ctx: set_rgb_status::Context<'_>) {
//     let rgb_driver = ctx.local.rgb_driver;
//     loop {
        
//         // let current_colors = ctx.shared.status_config.lock(|status| {
//         //     [
//         //         status.RBF,
//         //         status.HaLow,
//         //         status.Esp,
//         //         status.Infratracker,
//         //         status.Guard,
//         //         status.Jupiter,
//         //         status.ElectroMagnet,
//         //         status.Servos,
//         //         status.Jupiter_Avionics_Health,
//         //         status.Ejector_Health,
//         //         status.Odin_Compute_Health,
//         //         status.Odin_Pico_Health,
//         //     ]
//         // });


//         let dim_red     = RGB8::new(50, 0, 0);
//         let dim_green   = RGB8::new(0, 50, 0);
//         let dim_blue    = RGB8::new(0, 0, 50);

//         let dim_yellow  = RGB8::new(40, 40, 0);
//         let dim_cyan    = RGB8::new(0, 40, 40);
//         let dim_magenta = RGB8::new(40, 0, 40);

//         let dim_orange  = RGB8::new(50, 20, 0);
//         let dim_purple  = RGB8::new(25, 0, 50);
//         let dim_white   = RGB8::new(30, 30, 30);
//         let off         = RGB8::new(0, 0, 0);

//         let current_colors = [
//             dim_red,
//             dim_green,
//             dim_blue,

//             dim_yellow,
//             dim_cyan,
//             dim_magenta,

//             dim_orange,
//             dim_purple,
//             dim_white,
//             off,
//             dim_orange,
//             dim_purple,
//         ];

//         rgb_driver.write(current_colors.iter().cloned()).unwrap();



//         // let dim_red     = Color::rgb(50, 0, 0);
//         // let dim_green   = Color::rgb(0, 50, 0);
//         // let dim_blue    = Color::rgb(0, 0, 50);

//         // let dim_yellow  = Color::rgb(40, 40, 0);
//         // let dim_cyan    = Color::rgb(0, 40, 40);
//         // let dim_magenta = Color::rgb(40, 0, 40);

//         // let dim_orange  = Color::rgb(50, 20, 0);
//         // let dim_purple  = Color::rgb(25, 0, 50);
//         // let dim_white   = Color::rgb(30, 30, 30);
//         // let off         = Color::rgb(0, 0, 0);

//         // let current_colors = [
//         //      dim_red,
//         //     dim_green,
//         //     dim_blue,

//         //     dim_yellow,
//         //     dim_cyan,
//         //     dim_magenta,

//         //     dim_orange,
//         //     dim_purple,
//         //     dim_white,
//         //     off
//         // ];
//         // &mut rgb_driver.async_send_color(current_colors).await;

        
//         // cortex_m::interrupt::free(|_| { 

//         // rgb_driver.send_color([
        
//         // ]);
//         // });

//         // cortex_m::interrupt::free(|_| { 
//         //     rgb_driver.send_color(current_colors);
//         // });

//         // for color in current_colors {
//         //     // info!("Loop");
//         //     rgb_driver.send_color([color]);
//         //     // Mono::delay(3_u64.micros()).await;
//         // }
//         Mono::delay(1000_u64.millis()).await;

//         // rgb_driver.send_color(current_colors);
//         // Mono::delay(10_u64.millis()).await;

//         // Mono::delay(10_u64.millis()).await;
//     }
// }

