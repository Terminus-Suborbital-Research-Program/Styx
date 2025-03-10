use crate::hal;
use crate::{
    communications::link_layer::{Device, LinkLayerPayload, LinkPacket},
    utilities, Mono,
};
use bin_packets::packets::{CommandPacket, ConnectionTest};
use embedded_io::Write;
use fugit::ExtU64;
use icarus::{print, println};
use rtic::Mutex;
use rtic_monotonics::Monotonic;
use rtic_sync::channel::Receiver;

use crate::{
    app::*,
    communications::serial_handler::{HEAPLESS_STRING_ALLOC_LENGTH, MAX_USB_LINES},
};

pub async fn command_handler(
    mut ctx: command_handler::Context<'_>,
    mut reciever: Receiver<'static, heapless::String<HEAPLESS_STRING_ALLOC_LENGTH>, MAX_USB_LINES>,
) {
    use bin_packets::ApplicationPacket;
    use embedded_io::{Read as _, ReadReady as _};

    while let Ok(line) = reciever.recv().await {
        // Split into commands and arguments, on whitespace
        let mut parts = line.split_whitespace();

        // Get the command
        let command = parts.next().unwrap_or_default();

        match command {
            "usb-reboot" => {
                // Reboots to the USB bootloader interface
                println!(ctx, "Rebooting...");

                hal::reboot::reboot(
                    hal::reboot::RebootKind::BootSel {
                        picoboot_disabled: false,
                        msd_disabled: false,
                    },
                    hal::reboot::RebootArch::Normal,
                );
            }

            "set-servo" => {
                // Parse arg as int or fail
                let arg = parts
                    .next()
                    .unwrap_or_default()
                    .parse::<u32>()
                    .unwrap_or_default();
                //channel_b.set_duty_cycle_percent(0).unwrap();
                ctx.shared.ejector_driver.lock(|channel| {
                    //let cycle = min_duty + ((max_duty - min_duty) * arg) / 200;
                    channel.set_angle(arg as u16);
                });
            }

            "enable-servo" => {
                ctx.shared.ejector_driver.lock(|channel| {
                    channel.enable();
                });
            }

            "disable-servo" => {
                ctx.shared.ejector_driver.lock(|channel| {
                    channel.disable();
                });
            }

            "set-locking-servo" => {
                // Parse arg as int or fail
                let arg = parts
                    .next()
                    .unwrap_or_default()
                    .parse::<u32>()
                    .unwrap_or_default();
                //channel_b.set_duty_cycle_percent(0).unwrap();
                ctx.shared.locking_driver.lock(|channel| {
                    //let cycle = min_duty + ((max_duty - min_duty) * arg) / 200;
                    channel.set_angle(arg as u16);
                });
            }

            "enable-locking-servo" => {
                ctx.shared.locking_driver.lock(|channel| {
                    channel.enable();
                });
            }

            "disable-locking-servo" => {
                ctx.shared.locking_driver.lock(|channel| {
                    channel.disable();
                });
            }

            "transmit-test" => {
                let mut sequence_number: u8 = 0;
                let mut connection = ConnectionTest::Start;

                loop {
                    let packet =
                        ApplicationPacket::Command(CommandPacket::ConnectionTest(connection));
                    let link_packet = ctx
                        .shared
                        .radio_link
                        .lock(|device| device.construct_packet(packet, Device::Icarus));
                    let serialized =
                        bincode::encode_to_vec(&link_packet, bincode::config::standard()).unwrap();

                    ctx.shared.radio_link.lock(|device| {
                        device.device.write(&serialized).ok();
                    });

                    if connection == ConnectionTest::End {
                        break;
                    }

                    // Update the connection test sequence
                    connection = match connection {
                        ConnectionTest::Start => {
                            println!(ctx, "Starting Connection Test");
                            sequence_number = 0;
                            ConnectionTest::Sequence(sequence_number)
                        }

                        ConnectionTest::Sequence(_) => {
                            if sequence_number == 255 {
                                ConnectionTest::End
                            } else {
                                sequence_number += 1;
                                println!(ctx, "Sequence: {}", sequence_number);
                                ConnectionTest::Sequence(sequence_number)
                            }
                        }

                        ConnectionTest::End => {
                            println!(ctx, "Ending Connection Test");
                            ConnectionTest::Start
                        }
                    };

                    Mono::delay(200_u64.millis()).await;
                }
            }

            // Tests the hash of a linkpacket
            "packet-hash-test" => {
                // Two identical packets should have the same hash
                let mut packet = LinkPacket {
                    from_device: Device::Atmega,
                    to_device: Device::Atmega,
                    route_through: None,
                    payload: LinkLayerPayload::NODATA,
                    checksum: None,
                };

                let mut packet2 = LinkPacket {
                    from_device: Device::Atmega,
                    to_device: Device::Atmega,
                    route_through: None,
                    payload: LinkLayerPayload::NODATA,
                    checksum: None,
                };

                packet.set_checksum();
                packet2.set_checksum();

                println!(ctx, "Packet 1: {:?}", packet);
                println!(ctx, "Packet 2: {:?}", packet2);
                Mono::delay(1000_u64.millis()).await;
                println!(
                    ctx,
                    "{}, {}",
                    packet.checksum.unwrap(),
                    packet2.checksum.unwrap()
                );

                // Change a field and the hash should change
                packet2.from_device = Device::Pi;
                packet2.set_checksum();

                println!(ctx, "Packet 1: {:?}", packet);
                println!(ctx, "Packet 2: {:?}", packet2);
                Mono::delay(1000_u64.millis()).await;
                println!(
                    ctx,
                    "{}, {}",
                    packet.checksum.unwrap(),
                    packet2.checksum.unwrap()
                );
            }

            // Peeks at the buffer, but with hex
            "link-peek-hex" => {
                let buffer = ctx
                    .shared
                    .radio_link
                    .lock(|radio| radio.device.clone_buffer());

                for c in buffer.iter() {
                    print!(ctx, "{:02X} ", *c);
                    Mono::delay(10_u64.millis()).await;
                }
                println!(ctx, "");
            }

            // Peeks at the buffer, printing it to the console
            "link-peek" => {
                let buffer = ctx
                    .shared
                    .radio_link
                    .lock(|radio| radio.device.clone_buffer());

                for c in buffer.iter() {
                    print!(ctx, "{}", *c as char);
                    Mono::delay(10_u64.millis()).await;
                }
                println!(ctx, "");
            }

            // HC12 Configuration Utility
            "hc-configure" => {
                // Clear out the buffer, the HC12 often sends a bit of junk when
                // it goes into config mode
                println!(ctx, "Clearing Buffer");
                ctx.shared.radio_link.lock(|link| {
                    link.device.clear();
                    link.device.write("AT\n".as_bytes()).ok();
                });

                Mono::delay(500_u64.millis()).await;

                println!(ctx, "AT Command Sent");
                ctx.shared.radio_link.lock(|link| {
                    link.device.update().ok();
                    while link.device.read_ready().unwrap_or(false) {
                        let mut buffer = [0u8; 1];
                        link.device.read(&mut buffer).ok();
                        print!(ctx, "{}", buffer[0] as char);
                    }
                });

                // Set baudrate
                ctx.shared.radio_link.lock(|link| {
                    link.device.write("AT+B9600\n".as_bytes()).ok();
                });
                Mono::delay(500_u64.millis()).await;
                ctx.shared.radio_link.lock(|link| {
                    link.device.update().ok();
                    while link.device.read_ready().unwrap_or(false) {
                        let mut buffer = [0u8; 1];
                        link.device.read(&mut buffer).ok();
                        print!(ctx, "{}", buffer[0] as char);
                    }
                });

                // Set channel (100)
                ctx.shared.radio_link.lock(|link| {
                    link.device.write("AT+C100\n".as_bytes()).ok();
                });
                Mono::delay(500_u64.millis()).await;
                ctx.shared.radio_link.lock(|link| {
                    link.device.update().ok();
                    while link.device.read_ready().unwrap_or(false) {
                        let mut buffer = [0u8; 1];
                        link.device.read(&mut buffer).ok();
                        print!(ctx, "{}", buffer[0] as char);
                    }
                });

                // Set power to max (8)
                ctx.shared.radio_link.lock(|link| {
                    link.device.write("AT+P8\n".as_bytes()).ok();
                });
                Mono::delay(500_u64.millis()).await;
                ctx.shared.radio_link.lock(|link| {
                    link.device.update().ok();
                    while link.device.read_ready().unwrap_or(false) {
                        let mut buffer = [0u8; 1];
                        link.device.read(&mut buffer).ok();
                        print!(ctx, "{}", buffer[0] as char);
                    }
                });
            }

            "clock-freq" => {
                // Print the current clock frequency
                ctx.shared.clock_freq_hz.lock(|freq| {
                    println!(ctx, "Clock Frequency: {} Hz", freq);
                });
            }

            "sp" => {
                // Print the stack pointer
                println!(
                    ctx,
                    "Stack Pointer: 0x{:08X}",
                    utilities::arm::get_stack_pointer()
                );
            }

            _ => {
                println!(ctx, "Invalid command: {}", command);
            }
        }
    }
}
