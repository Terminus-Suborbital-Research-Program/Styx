use crate::utilities;
use embedded_io::Write;
use icarus::println;
use rtic::Mutex;
use rtic_sync::channel::Receiver;

use crate::{
    app::*,
    communications::serial_handler::{HEAPLESS_STRING_ALLOC_LENGTH, MAX_USB_LINES},
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
