use crate::{communications::link_layer::Device, hal, utilities};
use bin_packets::packets::ApplicationPacket;
use ejector::println;
use embedded_io::Write;
use fugit::{Duration, ExtU64};
use rtic::Mutex;
use rtic_monotonics::Monotonic;
use rtic_sync::channel::{Receiver, Sender};

use crate::{
    app::*,
    communications::serial_handler::{HEAPLESS_STRING_ALLOC_LENGTH, MAX_USB_LINES},
    Mono,
};

pub async fn command_handler(
    mut ctx: command_handler::Context<'_>,
    mut reciever: Receiver<'static, heapless::String<HEAPLESS_STRING_ALLOC_LENGTH>, MAX_USB_LINES>,
) {
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

            "phase" => {
                // Print the current phase
                let phase = ctx
                    .shared
                    .state_machine
                    .lock(|state_machine| state_machine.phase());

                println!(ctx, "Current Phase: {:?}", phase);
            }

            // "command-eject" => {
            //     // Send an ejection command to the ejector
            //     let packet = ApplicationPacket::Command(
            //         bin_packets::packets::CommandPacket::EjectorPhaseSet(
            //             bin_packets::phases::EjectorPhase::Ejection,
            //         ),
            //     );

            //     let link_packet = ctx
            //         .shared
            //         .radio_link
            //         .lock(|radio| radio.construct_packet(packet, Device::Ejector));

            //     ctx.shared.radio_link.lock(|radio| {
            //         radio.write_link_packet(link_packet).ok();
            //     });
            // }

            // "command-standby" => {
            //     // Send a standby command to the ejector
            //     let packet = ApplicationPacket::Command(
            //         bin_packets::packets::CommandPacket::EjectorPhaseSet(
            //             bin_packets::phases::EjectorPhase::Standby,
            //         ),
            //     );

            //     let link_packet = ctx
            //         .shared
            //         .radio_link
            //         .lock(|radio| radio.construct_packet(packet, Device::Ejector));

            //     ctx.shared.radio_link.lock(|radio| {
            //         radio.write_link_packet(link_packet).ok();
            //     });
            // }

            // Runs a ping test, pinging Ejector 'n' times
            // "ping" => {
            //     let n = parts.next().unwrap_or("1").parse::<u32>().unwrap_or(1);
            //     let mut recieved = 0;
            //     let mut sent = 0;

            //     // Suspend the packet handler
            //     ctx.shared
            //         .suspend_packet_handler
            //         .lock(|suspend| *suspend = true);

            //     while recieved < n {
            //         // Send a ping command
            //         let packet =
            //             ApplicationPacket::Command(bin_packets::packets::CommandPacket::Ping);
            //         let link_packet = ctx
            //             .shared
            //             .radio_link
            //             .lock(|radio| radio.construct_packet(packet, Device::Ejector));

            //         println!(ctx, "Sending Ping: {}", sent);
            //         ctx.shared.radio_link.lock(|radio| {
            //             radio.write_link_packet(link_packet).ok();
            //         });
            //         sent += 1;
            //         println!(ctx, "Sent: {}", sent);

            //         // Note the current time
            //         let start_time = Mono::now();

            //         // Wait for a reply, if we don't get one in 1s, we'll just send another
            //         let del: Duration<u64, 1, 1000000> = 1u64.secs();
            //         while Mono::now() - start_time < del {
            //             // Check for a reply
            //             if let Some(_packet) =
            //                 ctx.shared.radio_link.lock(|radio| radio.read_link_packet())
            //             {
            //                 recieved += 1;
            //                 println!(ctx, "Recieved: {}", recieved);
            //                 break;
            //             }

            //             Mono::delay(10_u64.millis()).await;
            //         }
            //         Mono::delay(30_u64.millis()).await;
            //     }

            //     println!(ctx, "Sent: {}, Recieved: {}", sent, recieved);
            //     // Percentage of packets recieved
            //     Mono::delay(300_u64.millis()).await;
            //     println!(ctx, "{}%", (recieved as f32 / sent as f32) * 100.0);

            //     // Resume the packet handler
            //     ctx.shared
            //         .suspend_packet_handler
            //         .lock(|suspend| *suspend = false);
            // }

            // // Configure manually
            // "hc-configure" => {
            //     hc12_programmer::spawn().ok();
            // }
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

pub async fn usb_console_reader(
    mut ctx: usb_console_reader::Context<'_>,
    mut command_sender: Sender<
        'static,
        heapless::String<HEAPLESS_STRING_ALLOC_LENGTH>,
        MAX_USB_LINES,
    >,
) {
    let mut buf = [0u8; 64];
    let mut command_buffer = heapless::String::<HEAPLESS_STRING_ALLOC_LENGTH>::new();

    let mut end_of_line = false;

    loop {
        ctx.shared.usb_device.lock(|usb_dev| {
            ctx.shared.usb_serial.lock(|serial| {
                if usb_dev.poll(&mut [serial]) {
                    // For the moment, we're just going to echo back the input, after a newline
                    match serial.read(&mut buf) {
                        Ok(count) if count > 0 => {
                            // Collect buffer into an array
                            let bytes = &buf[..count];
                            for byte in bytes.iter() {
                                // Conv to char
                                let c = *byte as char;

                                // Write to serial to echo
                                serial.write(&[*byte]).ok();

                                // Detect eol
                                if c == '\r' || c == '\n' {
                                    end_of_line = true;
                                    serial.write_all("\r\n".as_bytes()).ok();
                                }

                                if c == '\x08' || c == '\x7F' {
                                    command_buffer.pop();
                                    serial.write_all("\x08 \x08".as_bytes()).ok();
                                } else {
                                    // Append to buffer
                                    command_buffer.push(c).ok();
                                }
                            }
                        }

                        _ => {
                            // Ignore errors on read, assume it was just a desync
                        }
                    }
                }
            })
        });

        if end_of_line {
            end_of_line = false;
            // Send the command to the command handler
            command_sender.try_send(command_buffer.clone()).ok();
            command_buffer.clear();
        }

        // Wait for a bit to poll again
        Mono::delay(1000_u64.micros()).await;
    }
}

pub async fn usb_serial_console_printer(
    mut ctx: usb_serial_console_printer::Context<'_>,
    mut reciever: Receiver<'static, heapless::String<HEAPLESS_STRING_ALLOC_LENGTH>, MAX_USB_LINES>,
) {
    while let Ok(mut line) = reciever.recv().await {
        // If the line ends with a newline, pop it off, and then add a \r\n
        if line.ends_with('\n') {
            line.pop();
            line.push_str("\r\n").ok();
        }

        let mut wr_ptr = line.as_bytes();
        while !wr_ptr.is_empty() {
            let res = ctx.shared.usb_serial.lock(|serial| serial.write(wr_ptr));
            match res {
                Ok(len) => wr_ptr = &wr_ptr[len..],
                Err(_) => break,
            }
            Mono::delay(10_u64.millis()).await;
        }
    }
}
