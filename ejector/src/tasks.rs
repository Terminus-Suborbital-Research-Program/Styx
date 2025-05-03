use core::cmp::max;

use bin_packets::{devices::DeviceIdentifier, packets::status::Status, phases::EjectorPhase};
use defmt::info;
use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};
use fugit::ExtU64;
use rtic::Mutex;
use rtic_monotonics::Monotonic;

use crate::{app::*, Mono};

const START_CAMERA_DELAY: u64 = 1000; // 10k millis For testing, 250 for actual

pub async fn heartbeat(mut ctx: heartbeat::Context<'_>) {
    let mut sequence_number = 0;
    loop {
        _ = ctx.local.led.toggle();

        let status = Status::new(DeviceIdentifier::Icarus, now_timestamp(), sequence_number);

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
        info!("Cameras RBF Inhibited");
    }
}

pub async fn radio_read(mut ctx: radio_read::Context<'_>) {
    loop {
        match ctx.shared.radio.lock(|radio| radio.read_packet()) {
            Ok(potential) => {
                if let Some(packet) = potential {
                    info!("Read packet: {:?}", packet);
                    // Write down range
                    info!(
                        "Write results: {:?}",
                        ctx.shared
                            .downlink
                            .lock(|downlink| { downlink.write_into(packet).err() })
                    );
                }
            }

            Err(e) => {
                info!("Error reading packet: {:?}", e);
            }
        }

        Mono::delay(1000_u64.millis()).await;
    }
}

pub fn uart_interrupt(mut ctx: uart_interrupt::Context<'_>) {
    ctx.shared.radio.lock(|radio| {
        if let Err(e) = radio.update() {
            info!("Error updating radio: {:?}", e);
        }
    });
}

pub async fn state_machine_update(mut ctx: state_machine_update::Context<'_>) {
    let rbf_on_startup = ctx.shared.rbf_status.lock(|startup_status| *startup_status);

    loop {
        let wait_time = ctx
            .shared
            .state_machine
            .lock(|state_machine| state_machine.transition());

        let gp_state = ctx.shared.ejection_pin.lock(|pin| pin.is_high().unwrap());

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

                if gp_state {
                    ctx.shared.state_machine.lock(|machine| {
                        machine.set_phase(EjectorPhase::Ejection);
                    })
                }

                // 1000ms delay
                ctx.shared
                    .blink_status_delay_millis
                    .lock(|delay| *delay = 1000);
            }

            EjectorPhase::Ejection => {
                // Eject the deployable
                if !rbf_on_startup {
                    ctx.shared.ejector_servo.lock(|servo| {
                        servo.eject();
                    });
                    info!("Servo eject")
                } else {
                    info!("RBF Mode: ejection inhibited")
                }

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
