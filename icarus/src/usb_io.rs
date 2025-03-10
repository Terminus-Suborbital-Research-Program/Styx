use embedded_io::Write;
use fugit::ExtU64;
use rtic::Mutex;
use rtic_monotonics::Monotonic;
use rtic_sync::channel::{Receiver, Sender};

use crate::{
    app::*,
    communications::serial_handler::{HEAPLESS_STRING_ALLOC_LENGTH, MAX_USB_LINES},
    Mono,
};

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

        ctx.shared.usb_device.lock(|_usb_dev| {
            ctx.shared.usb_serial.lock(|serial| {
                let mut wr_ptr = line.as_bytes();
                while !wr_ptr.is_empty() {
                    match serial.write(wr_ptr) {
                        Ok(len) => wr_ptr = &wr_ptr[len..],
                        Err(_) => break,
                    }
                }
            })
        })
    }
}
