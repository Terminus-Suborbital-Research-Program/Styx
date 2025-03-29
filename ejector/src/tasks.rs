use core::cmp::max;

use bin_packets::{phases::EjectorPhase, ApplicationPacket, LinkPacket};
use bincode::config::standard;
use defmt::info;
use embedded_hal::digital::StatefulOutputPin;
use embedded_io::Write;
use fugit::ExtU64;
use rtic::Mutex;
use rtic_monotonics::Monotonic;

use crate::{app::*, Mono};

pub async fn incoming_packet_handler(ctx: incoming_packet_handler::Context<'_>) {
    Mono::delay(1000_u64.millis()).await;
}

pub async fn heartbeat(mut ctx: heartbeat::Context<'_>) {
    loop {
        _ = ctx.local.led.toggle();

        Mono::delay(
            ctx.shared
                .blink_status_delay_millis
                .lock(|delay| *delay)
                .millis(),
        )
        .await;
    }
}

pub async fn radio_heartbeat(mut ctx: radio_heartbeat::Context<'_>) {
    let mut packet_num = 0;
    loop {
        let packet = ApplicationPacket::EjectorStatus(bin_packets::EjectorStatus {
            phase: EjectorPhase::Hold,
            time_in_phase: 100,
            timestamp: 300,
            packet_number: packet_num,
        });

        let packet = LinkPacket::new(bin_packets::DeviceIdentifier::Ejector, bin_packets::DeviceIdentifier::Broadcast, packet);
        packet_num += 1;

        ctx.shared.radio.lock(|radio| {
            let mut buf = [0u8; 64];
            let bytes = bincode::encode_into_slice(packet, &mut buf, standard()).unwrap();

            radio.write_all(&buf[0..bytes]).ok();
        });

        Mono::delay(1000_u64.millis()).await;
    }
}

pub fn uart_interrupt(ctx: uart_interrupt::Context<'_>) {
    // ctx.shared.radio_link.lock(|radio| {
    //     radio.device.update().ok();
    // });
}

pub async fn state_machine_update(mut ctx: state_machine_update::Context<'_>) {
    loop {
        let wait_time = ctx
            .shared
            .state_machine
            .lock(|state_machine| state_machine.transition());

        match ctx
            .shared
            .state_machine
            .lock(|state_machine| state_machine.phase())
        {
            EjectorPhase::Standby => {
                // Hold the deployable
                ctx.shared.ejector_servo.lock(|servo| {
                    servo.hold();
                });

                // 1000ms delay
                ctx.shared
                    .blink_status_delay_millis
                    .lock(|delay| *delay = 1000);
            }

            EjectorPhase::Ejection => {
                // Eject the deployable
                ctx.shared.ejector_servo.lock(|servo| {
                    servo.eject();
                });
                // 200ms delay
                ctx.shared
                    .blink_status_delay_millis
                    .lock(|delay| *delay = 200);
            }

            EjectorPhase::Hold => {
                // Hold the deployable
                ctx.shared.ejector_servo.lock(|servo| {
                    servo.hold();
                });
                // 5000ms delay
                ctx.shared
                    .blink_status_delay_millis
                    .lock(|delay| *delay = 5000);
            }
        }

        // We should never wait less than 1ms, tbh
        Mono::delay(max(wait_time, 1).millis()).await;
    }
}

pub async fn hc12_programmer(ctx: hc12_programmer::Context<'_>) {
    info!("Programming HC12. Don't do this!");
}

// pub async fn radio_flush(mut ctx: radio_flush::Context<'_>) {
//     let mut on_board_baudrate: BaudRate = BaudRate::B9600;
//     let bytes_to_flush = 16;

//     loop {
//         ctx.shared.radio_link.lock(|radio| {
//             radio.device.flush(bytes_to_flush).ok();
//             on_board_baudrate = radio.device.get_baudrate();
//         });

//         // Need to wait wait the in-air baudrate, or the on-board baudrate
//         // whichever is slower

//         let mut slower =
//             core::cmp::min(on_board_baudrate.to_u32(), on_board_baudrate.to_in_air_bd());

//         // slower is bps, so /1000 to get ms
//         slower = slower / 1000;

//         // Delay for that times the number of bytes flushed
//         Mono::delay((slower as u64 * bytes_to_flush as u64).millis()).await;
//     }
// }
