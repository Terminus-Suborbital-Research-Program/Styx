use crate::{app::*, Mono};
use bin_packets::{devices::DeviceIdentifier, packets::status::Status};
use bincode::{config::standard, encode_into_slice};
use common::rbf::RbfIndicator;
use defmt::{debug, info, warn};
use embedded_hal::digital::{InputPin, StatefulOutputPin};
use embedded_io::Write;
use fugit::ExtU64;
use rtic::Mutex;
use rtic_monotonics::Monotonic;

/// Constant to prevent ejector from interfering with JUPITER's u-boot sequence
#[cfg(not(feature = "fast-startup"))]
const JUPITER_BOOT_LOCKOUT_TIME_SECONDS: u64 = 180;
#[cfg(feature = "fast-startup")]
const JUPITER_BOOT_LOCKOUT_TIME_SECONDS: u64 = 10;

pub async fn heartbeat(mut ctx: heartbeat::Context<'_>) {
    Mono::delay(JUPITER_BOOT_LOCKOUT_TIME_SECONDS.secs()).await;

    let mut sequence_number = 0;

    loop {
        ctx.shared.led.lock(|led| led.toggle().unwrap());

        debug!("Heartbeat: Sending Status packet");
        let status = Status::new(DeviceIdentifier::Ejector, now_timestamp(), sequence_number);

        ctx.shared
            .downlink_packets
            .lock(|q| q.push_back(status.into()).ok());

        sequence_number = sequence_number.wrapping_add(1);

        Mono::delay(300_u64.millis()).await;
    }
}

pub async fn radio_read(mut ctx: radio_read::Context<'_>) {
    let downlink = ctx.local.downlink;
    // Delay to avoid interference with JUPITER bootloader
    Mono::delay(JUPITER_BOOT_LOCKOUT_TIME_SECONDS.secs()).await;

    // Drain all available packets, one per lock to allow interruption
    let mut buffer = [0u8; 256];
    loop {
        ctx.shared.radio.lock(|x| {
            x.update().ok();

            if !x.frame_buffer().is_empty() {
                debug!("Buffer: {}", x.frame_buffer());
            }

            if !x.packet_buffer().is_empty() {
                debug!("Buffer: {}", x.packet_buffer());
            }

            for packet in x {
                info!("Got packet: {}", packet);
                let bytes = encode_into_slice(packet, &mut buffer, standard()).unwrap_or(0);
                downlink.write_all(&buffer[0..bytes]).ok();
            }
        });

        ctx.shared.downlink_packets.lock(|packets| {
            while let Some(packet) = packets.pop_front() {
                let bytes = encode_into_slice(packet, &mut buffer, standard()).unwrap_or(0);
                downlink.write_all(&buffer[0..bytes]).ok();
            }
        });

        Mono::delay(100.millis()).await;
    }
}

pub fn uart_interrupt(_ctx: uart_interrupt::Context<'_>) {
    radio_read::spawn().ok();
}

pub async fn ejector_sequencer(mut ctx: ejector_sequencer::Context<'_>) {
    let servo = ctx.local.ejector_servo;
    // Latch ejector servos closed
    servo.enable();
    servo.hold();

    let ejection_pin = ctx.local.ejection_pin;

    // Lockout for one minute to let JUPITER boot up
    warn!("Idling sequencer");
    Mono::delay(JUPITER_BOOT_LOCKOUT_TIME_SECONDS.secs()).await;
    info!("Sequencer unlocked, waiting for ejection signal");

    // Wait until ejection pin reads high
    while !ejection_pin.is_high().unwrap_or(false) {
        debug!("Ejector idling while waiting for ejection signal");
        Mono::delay(100_u64.millis()).await;
    }

    info!("Ejection signal high!");

    if ctx.shared.rbf.lock(|rbf| rbf.is_inserted()) {
        loop {
            Mono::delay(1000_u64.millis()).await;
            if ctx.shared.rbf.lock(|rbf| !rbf.is_inserted()) {
                break;
            }
        }
    }
    // Eject, wait 5 seconds, then retract
    info!("Ejecting!");
    servo.eject();
    Mono::delay(5000_u64.millis()).await;
    servo.hold();

    // Give three seconds to retract, then disable to save power
    Mono::delay(3000_u64.millis()).await;
    servo.disable();
    info!("Ejector disabled, servo disabled. Ejector sequencing complete.");
}
