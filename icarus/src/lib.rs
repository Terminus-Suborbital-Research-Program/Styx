#![no_std]
use embedded_hal_async::delay::DelayNs;
use rp235x_hal::{gpio, pac::I2C1, uart::UartPeripheral, I2C};

pub type UART0Bus = UartPeripheral<
    rp235x_hal::uart::Enabled,
    rp235x_hal::pac::UART0,
    (
        gpio::Pin<gpio::bank0::Gpio0, gpio::FunctionUart, gpio::PullDown>,
        gpio::Pin<gpio::bank0::Gpio1, gpio::FunctionUart, gpio::PullDown>,
    ),
>;

pub type I2CMainBus = I2C<
    I2C1,
    (
        gpio::Pin<gpio::bank0::Gpio14, gpio::FunctionI2c, gpio::PullUp>,
        gpio::Pin<gpio::bank0::Gpio15, gpio::FunctionI2c, gpio::PullUp>,
    ),
>;

pub type DelayTimer = rp235x_hal::Timer<rp235x_hal::timer::CopyableTimer1>;

// pub type BME280Device = BME280<rp235x_hal::I2C<I2C1, (gpio::Pin<gpio::bank0::Gpio14, gpio::FunctionI2c, gpio::PullUp>, gpio::Pin<gpio::bank0::Gpio15, gpio::FunctionI2c, gpio::PullUp>)>>;
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
