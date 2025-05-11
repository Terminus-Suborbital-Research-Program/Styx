use bin_packets::device::PacketIO;
use bin_packets::{devices::DeviceIdentifier, packets::status::Status};
use defmt::{info, trace, warn};
use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};
use fugit::ExtU64;
use rtic::Mutex;
use rtic_monotonics::Monotonic;

use crate::{app::*, Mono};

const START_CAMERA_DELAY: u64 = 1000; // 10k millis For testing, 250 for actual

pub async fn heartbeat(mut ctx: heartbeat::Context<'_>) {
    let mut sequence_number = 0;
    loop {
        ctx.shared.led.lock(|led| led.toggle().unwrap());

        let status = Status::new(DeviceIdentifier::Ejector, Mono::now().ticks(), sequence_number);

        let res = ctx
            .shared
            .downlink
            .lock(|downlink| downlink.write_into(status).err());

        if let Some(err) = res {
            info!("Error sending heartbeat: {:?}", err);
        }

        sequence_number = sequence_number.wrapping_add(1);

        Mono::delay(300_u64.millis()).await;
    }
}

pub async fn start_cameras(mut ctx: start_cameras::Context<'_>) {
    let rbf_on_startup = ctx.shared.rbf_status.lock(|startup_status| *startup_status);
    if !rbf_on_startup {
        info!("Camera Timer Starting");
        Mono::delay(START_CAMERA_DELAY.millis()).await;
        info!("Cameras on");
        ctx.local.cams.set_low().unwrap();
        loop {
            ctx.local.cams_led.toggle().unwrap();
            Mono::delay(1000.millis()).await;
        }
    } else {
        info!("RBF Inserted: Cameras disabled");
    }
}

pub async fn radio_read(mut ctx: radio_read::Context<'_>) {
    loop {
        // Drain all available packets, one per lock to allow interruptions
        loop {
            while let Some(packet) = ctx.shared.radio.lock(|radio| radio.read_packet()) {
                ctx.shared.led.lock(|led| led.toggle().unwrap());
                trace!("Got a packet form icarus! Packet: {:?}", packet);
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
    let rbf_on_startup = ctx.shared.rbf_status.lock(|startup_status| *startup_status);
    let servo = ctx.local.ejector_servo;
    // Latch ejector servos closed
    servo.enable();
    servo.hold();

    let ejection_pin = ctx.local.ejection_pin;

    // Wait until ejection pin reads high
    while !ejection_pin.is_high().unwrap_or(false) {
        Mono::delay(100_u64.millis()).await;
    }

    info!("Ejection signal high!");

    // If rbf was inserted on startup, exit immediately
    if rbf_on_startup {
        info!("RBF Inserted: Ejector disabled");
        return;
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
