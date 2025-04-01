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
