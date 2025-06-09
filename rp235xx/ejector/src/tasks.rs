use crate::{app::*, device_constants::SAMPLE_COUNT, Mono};
use bin_packets::{
    devices::DeviceIdentifier,
    packets::{status::Status, ApplicationPacket},
};
use bincode::{config::standard, decode_from_slice, encode_into_slice, error::DecodeError};
use defmt::{debug, info, warn};
use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};
use embedded_io::{Read, ReadReady, Write};
use fugit::ExtU64;
use heapless::{Deque, Vec};
use rtic::Mutex;
use rtic_monotonics::Monotonic;
use tinyframe::frame::Frame;

#[cfg(not(feature = "fast-startup"))]
const JUPITER_BOOT_LOCKOUT_TIME_SECONDS: u64 = 180;
/// Constant to prevent ejector from interfering with JUPITER's u-boot sequence
#[cfg(feature = "fast-startup")]
const JUPITER_BOOT_LOCKOUT_TIME_SECONDS: u64 = 10;

const SHUTDOWN_TIME_CAMERAS: u64 = 210;

pub async fn heartbeat(mut ctx: heartbeat::Context<'_>) {
    let onboard_led = ctx.local.onboard_led;

    let mut sequence_number = 0;

    // Still blink, but toggle as it is done
    loop {
        onboard_led.toggle().unwrap();

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

pub fn adc_irq(mut ctx: adc_irq::Context<'_>) {
    let sample = ctx.local.geiger_fifo.as_mut().unwrap().read();
    let i = *ctx.local.counter;

    ctx.shared.samples_buffer.lock(|buff| buff[i] = sample);
    *ctx.local.counter += 1;

    if i >= (SAMPLE_COUNT - 1) {
        *ctx.local.counter = 0;
        geiger_calculator::spawn().ok();
    }
}

pub async fn geiger_calculator(mut ctx: geiger_calculator::Context<'_>) {
    if Mono::now().duration_since_epoch().to_secs() > JUPITER_BOOT_LOCKOUT_TIME_SECONDS {
        let pulses = ctx
            .shared
            .samples_buffer
            .lock(|buf| buf.iter().filter(|x| **x > 10).count());

        if pulses > 0 {
            let packet = ApplicationPacket::GeigerData {
                timestamp_ms: Mono::now().duration_since_epoch().to_millis(),
                recorded_pulses: pulses as u16,
            };

            debug!("Recorded pulses! {}", pulses);

            if ctx
                .shared
                .downlink_packets
                .lock(|packets| packets.push_back(packet))
                .is_err()
            {
                warn!("Downlink packets full!");
            } else {
                info!("Geiger downlink packets queued: {} pulses", pulses);
            }
        }
    }
}

const SCRATCH: usize = 512;

pub async fn radio_read(mut ctx: radio_read::Context<'_>) {
    let downlink = ctx.local.downlink;
    let radio = ctx.local.radio;

    // Allow the JUPITER bootloader to finish its chatter.
    Mono::delay(JUPITER_BOOT_LOCKOUT_TIME_SECONDS.secs()).await;

    // Static working buffers.
    let mut tmp_buf = [0u8; SCRATCH];
    let mut frame_buf: Vec<u8, SCRATCH> = Vec::new();
    let mut packet_buf: Vec<u8, SCRATCH> = Vec::new();
    let mut outgoing_pkts: Deque<ApplicationPacket, 16> = Deque::new();

    loop {
        //------------------------------------------------------------------
        // 1. Pull any newly‑arrived UART bytes.
        //------------------------------------------------------------------
        if radio.read_ready().unwrap_or(false) {
            let space = SCRATCH - frame_buf.len();
            info!("Space: {}", space);
            if space > 0 {
                let n = radio.read(&mut tmp_buf[..space]).unwrap_or(0);
                frame_buf.extend_from_slice(&tmp_buf[..n]).ok();
            }
        }
        info!("Radio buffer size: {}", frame_buf.len());

        //------------------------------------------------------------------
        // 2. Decode TinyFrames until we run out of complete ones.
        //------------------------------------------------------------------
        'frame: loop {
            match Frame::decode_from_slice(&frame_buf) {
                Ok((frame, used)) => {
                    // Append payload to the packet staging buffer.
                    packet_buf.extend_from_slice(frame.payload()).ok();

                    // Shift unconsumed bytes left.
                    let len = frame_buf.len();
                    frame_buf.copy_within(used..len, 0);
                    frame_buf.truncate(len - used);
                }
                Err(tinyframe::Error::NotEnoughBytes) => break 'frame,
                Err(_) => {
                    // Corrupt trailing byte(s) – drop one and retry.
                    if !frame_buf.is_empty() {
                        let len = frame_buf.len();
                        frame_buf.copy_within(1..len, 0);
                        frame_buf.truncate(len - 1);
                    }
                    info!("Unexpected frame error, dropping byte");
                }
            }
        }
        info!("Frame Sent");

        let mut enc_buf = [0u8; SCRATCH];
        ctx.shared.downlink_packets.lock(|packets| {
            while let Some(packet) = packets.pop_front() {
                ctx.local.packet_led.toggle().ok();
                if let Ok(sz) = encode_into_slice(packet, &mut enc_buf, standard()) {
                    let _ = downlink.write_all(&enc_buf[..sz]);
                }
                info!("Sent packet: {}", packet);
            }
        });
        //------------------------------------------------------------------
        // 3. Decode application‑level packets.
        //------------------------------------------------------------------
        'packet: loop {
            match decode_from_slice::<ApplicationPacket, _>(&packet_buf, standard()) {
                Ok((pkt, used)) => {
                    outgoing_pkts.push_back(pkt).ok();

                    let len = packet_buf.len();
                    packet_buf.copy_within(used..len, 0);
                    packet_buf.truncate(len - used);
                    info!("Packet decoded: {}", pkt);
                }
                Err(DecodeError::UnexpectedEnd { .. }) => break 'packet,
                Err(_) => {
                    if !packet_buf.is_empty() {
                        let len = packet_buf.len();
                        packet_buf.copy_within(1..len, 0);
                        packet_buf.truncate(len - 1);
                    }
                }
            }
        }

        //------------------------------------------------------------------
        // 4. Flush any packets that are ready for the downlink.
        //------------------------------------------------------------------
        while let Some(pkt) = outgoing_pkts.pop_front() {
            if let Ok(sz) = encode_into_slice(pkt, &mut enc_buf, standard()) {
                let _ = downlink.write_all(&enc_buf[..sz]);
                info!("Sent packet: {:?}", pkt);
            }
        }
        Mono::delay(100.millis()).await;
    }
}

pub async fn camera_sequencer(ctx: camera_sequencer::Context<'_>) {
    // T+70, drive the cameras high
    Mono::delay(250.secs()).await;
    info!("Activating cameras!");
    ctx.local.camera_mosfet.set_high().ok();
    Mono::delay(SHUTDOWN_TIME_CAMERAS.secs()).await;
    info!("Shutting down cameras!");
    ctx.local.camera_mosfet.set_low().ok();
}

pub async fn ejector_sequencer(ctx: ejector_sequencer::Context<'_>) {
    let servo = ctx.local.ejector_servo;
    // Latch ejector servos closed
    servo.enable();
    servo.hold();

    let ejection_pin = ctx.local.ejection_pin;

    // Lockout for one minute to let JUPITER boot up
    warn!("Idling sequencer");
    Mono::delay(JUPITER_BOOT_LOCKOUT_TIME_SECONDS.secs()).await;
    ctx.local.arming_led.set_low().ok();
    info!("Sequencer unlocked, waiting for ejection signal");

    // Wait until ejection pin reads high
    while !ejection_pin.is_high().unwrap_or(false) {
        debug!("Ejector idling while waiting for ejection signal");
        Mono::delay(100_u64.millis()).await;
    }

    info!("Ejection signal high!");

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
