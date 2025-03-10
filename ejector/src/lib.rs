#![no_std]

// Custom macro for printing to the serial console
#[macro_export]
macro_rules! println {
    ($ctx:expr, $($arg:tt)*) => {{
        $ctx.shared.serial_console_writer.lock(|writer| {
            use core::fmt::Write; // Ensure Write trait is in scope
            let _ = writeln!(writer, $($arg)*);
        });
    }};
}

#[macro_export]
macro_rules! print {
    ($ctx:expr, $($arg:tt)*) => {{
        $ctx.shared.serial_console_writer.lock(|writer| {
            use core::fmt::Write; // Ensure Write trait is in scope
            let _ = write!(writer, $($arg)*);
        });
    }};
}
