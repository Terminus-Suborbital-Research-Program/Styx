use core::fmt::{self, Write as FmtWrite};
use embedded_hal::digital::OutputPin;
use embedded_io::{Read, ReadReady, Write, WriteReady};
use fugit::{HertzU32, RateExtU32};
use heapless::{Deque, String as HString, Vec};
use rp235x_hal::{
    gpio,
    uart::{DataBits, Enabled, StopBits, UartConfig, UartDevice, UartPeripheral, ValidUartPinout},
};

// UART Type
pub type GPIO10 = gpio::Pin<gpio::bank0::Gpio10, gpio::FunctionSioOutput, gpio::PullDown>;
pub type UART1Bus = UartPeripheral<
    rp235x_hal::uart::Enabled,
    rp235x_hal::pac::UART1,
    (
        gpio::Pin<gpio::bank0::Gpio8, gpio::FunctionUart, gpio::PullDown>,
        gpio::Pin<gpio::bank0::Gpio9, gpio::FunctionUart, gpio::PullDown>,
    ),
>;

// Clock frequency of the RP235x is 150_000_000Hz
const CLOCK_FREQ: u32 = 150_000_000;

// A trait to allow reconfiguration of a UART’s baudrate.
// Consumes and re-enables the UART if the hardware requires it.
pub trait UartReconfigurable: Sized {
    fn set_baudrate(self, baudrate: BaudRate, frequency: HertzU32) -> Result<Self, HC12Error>;
}

impl<D, P> UartReconfigurable for UartPeripheral<Enabled, D, P>
where
    D: UartDevice,
    P: ValidUartPinout<D>,
{
    fn set_baudrate(self, baudrate: BaudRate, frequency: HertzU32) -> Result<Self, HC12Error> {
        let peripheral = self.disable(); // move out of Enabled state
        let baudrate_u32 = baudrate.to_u32();

        // Re-enable with new baud rate
        let peripheral = peripheral
            .enable(
                UartConfig::new(baudrate_u32.Hz(), DataBits::Eight, None, StopBits::One),
                frequency,
            )
            .map_err(|_err| HC12Error::UartConfigError)?; // map the driver’s error to our own

        Ok(peripheral)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HC12Mode {
    Normal,
    Configuration,
}

// Supported baud rates for the HC12 module.
#[derive(Debug, Copy, Clone)]
#[repr(u32)]
#[allow(dead_code)]
pub enum BaudRate {
    B1200 = 1200,
    B2400 = 2400,
    B4800 = 4800,
    B9600 = 9600,
    B19200 = 19200,
    B38400 = 38400,
    B57600 = 57600,
    B115200 = 115200,
}

impl BaudRate {
    pub fn to_u32(&self) -> u32 {
        *self as u32
    }

    pub fn from_u32(baud: u32) -> Result<BaudRate, HC12Error> {
        match baud {
            1200 => Ok(BaudRate::B1200),
            2400 => Ok(BaudRate::B2400),
            4800 => Ok(BaudRate::B4800),
            9600 => Ok(BaudRate::B9600),
            19200 => Ok(BaudRate::B19200),
            38400 => Ok(BaudRate::B38400),
            57600 => Ok(BaudRate::B57600),
            115200 => Ok(BaudRate::B115200),
            _ => Err(HC12Error::InvalidBaudrate),
        }
    }

    // In-air baudrate equivalent of the physical baudrate
    pub fn to_in_air_bd(self) -> u32 {
        match self {
            BaudRate::B1200 => 5000,
            BaudRate::B2400 => 5000,
            BaudRate::B4800 => 15000,
            BaudRate::B9600 => 15000,
            BaudRate::B19200 => 58000,
            BaudRate::B38400 => 58000,
            BaudRate::B57600 => 236000,
            BaudRate::B115200 => 236000,
        }
    }
}

impl fmt::Display for BaudRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let baud_str = match self {
            BaudRate::B1200 => "1200",
            BaudRate::B2400 => "2400",
            BaudRate::B4800 => "4800",
            BaudRate::B9600 => "9600",
            BaudRate::B19200 => "19200",
            BaudRate::B38400 => "38400",
            BaudRate::B57600 => "57600",
            BaudRate::B115200 => "115200",
        };
        f.write_str(baud_str)
    }
}

// Error types for the HC12 driver.
#[derive(Debug)]
pub enum HC12Error {
    // Error toggling the config pin
    ConfigPinError,

    // Invalid baud rate supplied (or unsupported by the driver)
    InvalidBaudrate,

    // Failed reconfiguring UART
    UartConfigError,

    // UART misconfiguration—e.g., missing peripheral
    UartMisconfigured,

    // Error writing to UART or buffer
    WriteError,

    // Error reading from UART or buffer
    ReadError,

    // Outgoing buffer is full
    BufferFull,

    // Incoming buffer is empty
    BufferEmpty,

    // Attempted an operation in the wrong mode
    WrongMode,

    // Specified RF channel is out of allowable range
    InvalidChannel,

    // Specified RF power level is out of allowable range
    InvalidPower,

    // Hal error
    HalError(rp235x_hal::uart::Error),
}

#[allow(dead_code)]
pub struct HC12<Uart, ConfigPin> {
    uart: Option<Uart>,
    config_pin: ConfigPin,
    mode: HC12Mode,
    baudrate: BaudRate,
    incoming_buffer: Deque<u8, 128>,
    outgoing_buffer: Deque<u8, 128>,
}

impl<Uart, ConfigPin> HC12<Uart, ConfigPin> {
    pub fn bytes_available(&self) -> usize {
        self.incoming_buffer.len()
    }

    // Clears the incoming buffer
    pub fn clear(&mut self) {
        self.incoming_buffer.clear();
    }

    // Clones the buffer as a string
    pub fn clone_buffer(&self) -> Vec<u8, 128> {
        self.incoming_buffer.clone().iter().cloned().collect()
    }

    // Drops n bytes from the front of the buffer
    pub fn drop_bytes(&mut self, n: usize) {
        for _ in 0..n {
            self.incoming_buffer.pop_front();
        }
    }

    // Checks if the incoming buffer contains "OK"
    pub fn check_ok(&self) -> bool {
        self.contains("OK")
    }

    // Checks if the buffer contains some &str
    pub fn contains(&self, s: &str) -> bool {
        let cloned_buffer = self.incoming_buffer.clone();
        let mut buffer = HString::<128>::new();

        for c in cloned_buffer {
            let _ = buffer.push(c as char);
        }

        buffer.contains(s)
    }

    // Similar, but for a char
    pub fn contains_char(&self, c: char) -> bool {
        let cloned_buffer = self.incoming_buffer.clone();
        let mut buffer = HString::<128>::new();

        for c in cloned_buffer {
            let _ = buffer.push(c as char);
        }

        buffer.contains(c)
    }

    // Returns a clone of the incoming buffer as a String
    pub fn get_buffer(&self) -> HString<128> {
        let cloned_buffer = self.incoming_buffer.clone();
        let mut buffer = HString::<128>::new();

        for c in cloned_buffer {
            let _ = buffer.push(c as char);
        }

        buffer
    }

    // Returns all characters up to and including a character, otherwise None
    pub fn read_until(&mut self, c: u8) -> Option<HString<128>> {
        match self.contains_char(c as char) {
            false => None,

            true => {
                let mut buffer = HString::<128>::new();
                while let Some(b) = self.incoming_buffer.pop_front() {
                    buffer.push(b as char).ok()?;
                    if b == c {
                        break;
                    }
                }
                Some(buffer)
            }
        }
    }

    // Number of bytes available to write to the outgoing buffer
    pub fn max_bytes_to_write(&self) -> usize {
        128 - self.outgoing_buffer.len()
    }

    // Reads a line from the incoming buffer
    #[inline]
    pub fn read_line(&mut self) -> Option<HString<128>> {
        self.read_until(b'\n')
    }

    // Gets the current baudrate
    pub fn get_baudrate(&self) -> BaudRate {
        self.baudrate
    }
}

impl embedded_io::Error for HC12Error {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::OutOfMemory
    }
}

impl<Uart, ConfigPin> embedded_io::ErrorType for HC12<Uart, ConfigPin> {
    type Error = HC12Error;
}

// Impliment the Write/Read traits on the HC12
impl<Uart, ConfigPin> Write for HC12<Uart, ConfigPin> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, HC12Error> {
        let mut written: usize = 0;
        for byte in buf {
            match self.outgoing_buffer.push_back(*byte) {
                Ok(_) => written += 1,
                Err(_) => return Err(HC12Error::BufferFull),
            }
            written += 1;
        }
        Ok(written)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        // We don't actually do anything here, as this is actually non-blocking
        Ok(())
    }
}

impl<Uart, ConfigPin> Read for HC12<Uart, ConfigPin> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, HC12Error> {
        let mut read: usize = 0;
        for byte in buf {
            match self.incoming_buffer.pop_front() {
                Some(b) => {
                    *byte = b;
                    read += 1;
                }
                None => break,
            }
        }
        Ok(read)
    }
}

impl<Uart, ConfigPin> WriteReady for HC12<Uart, ConfigPin> {
    fn write_ready(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.outgoing_buffer.is_full())
    }
}

impl<Uart, ConfigPin> ReadReady for HC12<Uart, ConfigPin> {
    fn read_ready(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.incoming_buffer.is_empty())
    }
}

impl<Uart, ConfigPin> HC12<Uart, ConfigPin>
where
    Uart: Read + Write + ReadReady + WriteReady + UartReconfigurable,
    ConfigPin: OutputPin,
{
    fn reconfigure_uart_baudrate(
        &mut self,
        baudrate: BaudRate,
        frequency: HertzU32,
    ) -> Result<(), HC12Error> {
        // Take ownership of the UART peripheral
        let uart = self.uart.take().ok_or(HC12Error::UartMisconfigured)?;

        // Change the baudrate by consuming the UART peripheral
        let uart = uart.set_baudrate(baudrate, frequency)?;

        // Return the UART peripheral to the HC12 struct
        self.uart = Some(uart);

        Ok(())
    }

    fn set_hc12_baudrate(&mut self, baudrate: BaudRate) -> Result<(), HC12Error> {
        // Go into configuration mode
        self.set_mode(HC12Mode::Configuration)?;

        // Set the baudrate via "AT+Bxxxx"
        let mut baud_str = HString::<10>::new();
        write!(baud_str, "B{}", baudrate.to_u32()).map_err(|_| HC12Error::WriteError)?;

        self.send_at(&baud_str)?;

        // Exit configuration mode
        self.set_mode(HC12Mode::Normal)?;

        Ok(())
    }

    // Change the baudrate of the HC12 module AND the local UART peripheral.
    pub fn set_baudrate(
        &mut self,
        baudrate: BaudRate,
        frequency: HertzU32,
    ) -> Result<(), HC12Error> {
        // 1. Configure the remote HC12 module to the new baud rate
        self.set_hc12_baudrate(baudrate)?;

        // 2. Reconfigure our UART peripheral to match
        self.reconfigure_uart_baudrate(baudrate, frequency)?;

        Ok(())
    }

    // Creates and initializes a new HC12 driver object.
    pub fn new(uart: Uart, mut config_pin: ConfigPin) -> Result<Self, HC12Error> {
        // Attempt to set the HC12 module to normal mode (config pin = HIGH)
        config_pin.set_high().ok(); // Infallible

        let mut hc12 = HC12 {
            uart: Some(uart),
            config_pin,
            mode: HC12Mode::Normal,
            baudrate: BaudRate::B9600,
            incoming_buffer: Deque::new(),
            outgoing_buffer: Deque::new(),
        };

        hc12.set_mode(HC12Mode::Normal)?;

        Ok(hc12)
    }

    // Flushes the outgoing buffer in a non-blocking way, up to `max_bytes` bytes at a time
    // Returns `Ok(true)` if the buffer is empty after flushing.
    pub fn flush(&mut self, max_bytes: usize) -> Result<bool, HC12Error> {
        let uart = self.uart.as_mut().ok_or(HC12Error::WriteError)?;

        // Single-byte writes while writing is ready
        while let Some(c) = self.outgoing_buffer.pop_front() {
            if uart.write_ready().unwrap_or(false) {
                uart.write(&[c]).map_err(|_| HC12Error::WriteError)?;
            } else {
                // If the UART isn’t ready, push the byte back onto the buffer
                self.outgoing_buffer
                    .push_front(c)
                    .map_err(|_| HC12Error::BufferFull)?;
                break;
            }

            // If we’ve written the maximum number of bytes, stop
            if self.outgoing_buffer.is_empty()
                || max_bytes > 0 && self.outgoing_buffer.len() >= max_bytes
            {
                break;
            }
        }

        Ok(self.outgoing_buffer.is_empty())
    }

    // Reads any available bytes from the UART into the incoming buffer in a non-blocking way.
    pub fn update(&mut self) -> Result<(), HC12Error> {
        let uart = self.uart.as_mut().ok_or(HC12Error::ReadError)?;

        // Single-byte reads. If partial reads are possible, it’s either 0 or 1 byte for us.
        while uart.read_ready().unwrap_or(false) {
            let mut buff = [0u8; 1];
            uart.read(&mut buff).map_err(|_| HC12Error::ReadError)?;
            match self.incoming_buffer.push_back(buff[0]) {
                Ok(_) => (),
                Err(_) => {
                    // Popp off the first byte if the buffer is full, and push the new byte
                    self.incoming_buffer.pop_front();
                    self.incoming_buffer
                        .push_back(buff[0])
                        .map_err(|_| HC12Error::BufferFull)?;
                }
            }
        }

        Ok(())
    }

    // Sets the HC12 module to a mode (Normal or Configuration).
    pub fn set_mode(&mut self, mode: HC12Mode) -> Result<(), HC12Error> {
        match mode {
            HC12Mode::Normal => {
                self.config_pin
                    .set_high()
                    .map_err(|_| HC12Error::ConfigPinError)?;
                self.mode = HC12Mode::Normal;
            }
            HC12Mode::Configuration => {
                self.config_pin
                    .set_low()
                    .map_err(|_| HC12Error::ConfigPinError)?;

                // Per HC12 spec: configuration mode is always 9600 baud
                self.reconfigure_uart_baudrate(BaudRate::B9600, CLOCK_FREQ.Hz())?;
                self.mode = HC12Mode::Configuration;
            }
        }
        Ok(())
    }

    // Sends an AT command to the HC12 module (must be in Configuration mode).
    pub fn send_at(&mut self, command: &str) -> Result<(), HC12Error> {
        if self.mode == HC12Mode::Normal {
            return Err(HC12Error::WrongMode);
        }

        // Clear buffers to ensure fresh data
        self.outgoing_buffer.clear();
        self.incoming_buffer.clear();

        self.write("AT+".as_bytes())?;
        self.write(command.as_bytes())?;
        self.write("\n".as_bytes())?;

        self.flush(128)?;
        Ok(())
    }

    // Sends an AT check command ("AT") to the HC12 module (must be in Configuration mode).
    pub fn check_at(&mut self) -> Result<(), HC12Error> {
        if self.mode == HC12Mode::Normal {
            return Err(HC12Error::WrongMode);
        }

        self.outgoing_buffer.clear();

        self.write("AT\n".as_bytes())?;
        self.flush(128)?;
        Ok(())
    }

    // Sets the channel of the HC12 module (0..127).
    pub fn set_channel(&mut self, channel: u8) -> Result<(), HC12Error> {
        if channel > 127 {
            return Err(HC12Error::InvalidChannel);
        }

        let mut channel_str = HString::<10>::new();
        write!(channel_str, "C{}", channel).map_err(|_| HC12Error::WriteError)?;
        self.send_at(&channel_str)
    }

    // Sets the power of the HC12 module (1..8).
    pub fn set_power(&mut self, power: u8) -> Result<(), HC12Error> {
        if !(1..=8).contains(&power) {
            return Err(HC12Error::InvalidPower);
        }

        let mut power_str = HString::<10>::new();
        write!(power_str, "P{}", power).map_err(|_| HC12Error::WriteError)?;
        self.send_at(&power_str)
    }
}
