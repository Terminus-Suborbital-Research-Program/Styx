use crate::{app::*, Mono};
use bin_packets::device::PacketIO;
use bin_packets::{devices::DeviceIdentifier, packets::status::Status};
use common::rbf::RbfIndicator;
use defmt::{debug, info, trace, warn};
use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};
use fugit::{Duration, ExtU64};
use rp235x_hal::reboot::{self, RebootArch, RebootKind};
use rtic::Mutex;
use rtic_monotonics::Monotonic;

const START_CAMERA_DELAY: u64 = 1000; // 10k millis For testing, 250 for actual

pub async fn heartbeat(mut ctx: heartbeat::Context<'_>) {
    let mut sequence_number = 0;

    // Set initial time
    let task_start_time = Mono::now();
    // Delay sending packets for one minute
    let delay: Duration<u64, 1, 1000000> = 1_u64.minutes();

    loop {
        ctx.shared.led.lock(|led| led.toggle().unwrap());

        if Mono::now() - task_start_time > delay {
            debug!("Heartbeat: Sending Status packet");
            let status = Status::new(DeviceIdentifier::Ejector, now_timestamp(), sequence_number);

            let res = ctx
                .shared
                .downlink
                .lock(|downlink| downlink.write_into(status).err());

            if let Some(err) = res {
                info!("Error sending heartbeat: {:?}", err);
            }
        }

        sequence_number = sequence_number.wrapping_add(1);

        Mono::delay(300_u64.millis()).await;
    }
}

pub async fn rbf_monitor(mut ctx: rbf_monitor::Context<'_>) {
    loop {
        let inserted = ctx.shared.rbf.lock(|rbf| rbf.is_inserted());
        let inserted_at_boot = ctx.shared.rbf.lock(|rbf| rbf.inhibited_at_init());
        if ctx.shared.rbf.lock(|rbf| rbf.is_inserted()) {
            //info!("Inhibited, waiting for ejector inhibit to be removed");
            ctx.local.rbf_led.set_low().unwrap();
        }
        
        if !inserted && inserted_at_boot {
            warn!("RBF Removed, rebooting system into uninhibited state");
            Mono::delay(100_u64.millis()).await;
            reboot::reboot(RebootKind::Normal, RebootArch::Arm);
        }
        Mono::delay(1000_u64.millis()).await;
    }
}

pub async fn start_cameras(mut ctx: start_cameras::Context<'_>) {
    info!("Camera Timer Starting");
    Mono::delay(START_CAMERA_DELAY.millis()).await;

    info!("Camera Timer Fulfilled");
    loop {
        if ctx.shared.rbf.lock(|rbf| rbf.is_inserted()) {
            debug!("Inhibited, waiting for ejector inhibit to be removed");
            // High to disable cams
            ctx.local.cams.set_high().unwrap();
            ctx.local.cams_led.set_high().unwrap();
        } else {
            debug!("RBF Not  Inhibited");

            ctx.local.cams_led.toggle().unwrap();
            ctx.local.cams.set_low().unwrap();
        }

        Mono::delay(1000.millis()).await;
    }
}

pub async fn radio_read(mut ctx: radio_read::Context<'_>) {
    Mono::delay(1_u64.minutes()).await; // Delay to avoid interference with JUPITER bootloader
    loop {
        // Drain all available packets, one per lock to allow interruptions
        loop {
            while let Some(packet) = ctx.shared.radio.lock(|radio| radio.read_packet()) {
                ctx.shared.led.lock(|led| led.toggle().unwrap());
                debug!("Got a packet form icarus! Packet: {:?}", packet);
                // Write down range
                if let Err(e) = ctx
                    .shared
                    .downlink
                    .lock(|downlink| downlink.write_into(packet))
                {
                    warn!("Error writing packet: {:?}", e);
                }
            }
            Mono::delay(1000_u64.millis()).await;
        }
    }
}

pub fn uart_interrupt(mut ctx: uart_interrupt::Context<'_>) {
    ctx.shared.radio.lock(|radio| {
        if let Err(e) = radio.update() {
            info!("Error updating radio: {:?}", e);
        }
    });
}

pub async fn ejector_sequencer(mut ctx: ejector_sequencer::Context<'_>) {
    let servo = ctx.local.ejector_servo;
    // Latch ejector servos closed
    servo.enable();
    servo.hold();

    let ejection_pin = ctx.local.ejection_pin;

    // Lockout for one minute to let JUPITER boot up
    info!("Idling sequencer");
    Mono::delay(1_u64.minutes()).await;
    info!("Sequencer unlocked, waiting for ejection signal");

    // Wait until ejection pin reads high
    while !ejection_pin.is_high().unwrap_or(false) {
        Mono::delay(10_u64.millis()).await;
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
