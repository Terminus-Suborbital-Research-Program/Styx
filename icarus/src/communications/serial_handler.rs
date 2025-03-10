use core::fmt::{self, Write};
use heapless::String;
use rtic_sync::channel::Sender;

// Max number of lines to buffer in the USB console
pub const MAX_USB_LINES: usize = 5;
// Default length of buffer strings used
pub const HEAPLESS_STRING_ALLOC_LENGTH: usize = 128;
// Typedef for heapless string
pub type HeaplessString = String<HEAPLESS_STRING_ALLOC_LENGTH>;

pub struct SerialWriter {
    sender: Sender<'static, heapless::String<HEAPLESS_STRING_ALLOC_LENGTH>, MAX_USB_LINES>,
}

impl SerialWriter {
    pub fn new(
        sender: Sender<'static, heapless::String<HEAPLESS_STRING_ALLOC_LENGTH>, MAX_USB_LINES>,
    ) -> Self {
        SerialWriter { sender }
    }
}

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Convert the string to a heapless string
        let mut heapless_string = HeaplessString::new();

        if heapless_string.push_str(s).is_err() {
            return Err(fmt::Error);
        }

        if self.sender.try_send(heapless_string).is_err() {
            return Err(fmt::Error);
        }

        Ok(())
    }
}
