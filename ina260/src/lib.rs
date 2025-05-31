#![no_std]

use cast::{i32, u16, u32};

// TI INA260 Current Sensor
#[cfg(feature = "defmt")]
use defmt::error;
#[cfg(feature = "async")]
use embedded_hal_async::delay::DelayNs;
#[cfg(feature = "async")]
use embedded_hal_async::i2c::I2c as AsyncI2c;

#[cfg(feature = "async")]
pub struct AsyncINA260<I2C, Delay> {
    i2c: I2C,
    pub address: u8,
    _marker: core::marker::PhantomData<I2C>,
    state: u16,
    delay: Delay,
}

impl<I2C: AsyncI2c, D> AsyncINA260<I2C, D>
where
    I2C: AsyncI2c,
    D: DelayNs,
{
    /// Create a new INA260 instance
    ///
    /// # Arguments
    ///
    /// * `i2c` - The I2C peripheral to use
    /// * `address` - The I2C address of the INA260
    pub fn new(i2c: I2C, address: u8, delay: D) -> Self {
        AsyncINA260 {
            i2c,
            address,
            delay,
            _marker: core::marker::PhantomData,
            state: OperMode::SCBVC.bits()
                | Averaging::AVG64.bits()
                | SCConvTime::MS8_244.bits()
                | BVConvTime::MS8_244.bits(),
        }
    }

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        self.write_register(Register::CONFIG, 0x8000).await
    }

    async fn write_register(&mut self, register: Register, data: u16) -> Result<(), I2C::Error> {
        self.i2c
            .write(
                self.address,
                &[register.addr(), (data >> 8) as u8, (data & 255) as u8],
            )
            .await
    }

    async fn read_reg(&mut self, reg: Register) -> Result<[u8; 2], I2C::Error> {
        let mut buf = [0; 2];
        self.i2c
            .write_read(self.address, &[reg.addr()], &mut buf)
            .await?;
        Ok(buf)
    }

    /// Read a 16 bit unsigned integer from a register
    async fn read_reg_u16(&mut self, reg: Register) -> Result<u16, I2C::Error> {
        let buf = self.read_reg(reg).await?;
        Ok(u16::from_be_bytes(buf))
    }

    /// Read a 16-bit signed integer from a register
    async fn read_reg_i16(&mut self, reg: Register) -> Result<i16, I2C::Error> {
        let buf = self.read_reg(reg).await?;
        Ok(i16::from_be_bytes(buf))
    }

    /// Change the Mask/Enable mode of the INA260
    ///
    /// The Mask/Enable Register selects the function that is enabled to control the ALERT pin as well as how that pin
    /// functions. If multiple functions are enabled, the highest significant bit position Alert Function (D15-D11) takes
    /// priority and responds to the Alert Limit Register.
    #[inline(always)]
    pub async fn set_mask_enable(&mut self, m: MaskEnable) -> Result<(), I2C::Error> {
        self.write_register(Register::MASK_ENABLE, m.bits()).await
    }

    /// Set the alert limit of the INA260
    ///
    /// The Alert Limit Register contains the value used to compare to the register selected in the Mask/Enable Register
    /// to determine if a limit has been exceeded. The format for this register will match the format of the register that is
    /// selected for comparison.
    #[inline(always)]
    pub async fn set_alert_limit(&mut self, limit: u16) -> Result<(), I2C::Error> {
        self.write_register(Register::ALERT_LIMIT, limit).await
    }

    /// Change the averaging mode of the INA260
    #[inline(always)]
    pub async fn set_averaging_mode(&mut self, a: Averaging) -> Result<(), I2C::Error> {
        let bits = a.bits();
        let state = (self.state & !Averaging::AVG1024.bits()) | bits;
        let result = self.write_register(Register::CONFIG, state).await;
        self.state = state;
        result
    }

    /// Change the operating mode of the INA260. Please note that if you change to Triggered mode,
    /// you'll have to call this method again each time you would like to get a new sample.
    #[inline(always)]
    pub async fn set_operating_mode(&mut self, o: OperMode) -> Result<(), I2C::Error> {
        let bits = o.bits();
        let state = (self.state & !OperMode::SCBVC.bits()) | bits;
        let result = self.write_register(Register::CONFIG, state).await;
        self.state = state;
        result
    }

    /// Change the shut current conversion time
    #[inline(always)]
    pub async fn set_scconvtime_mode(&mut self, s: SCConvTime) -> Result<(), I2C::Error> {
        let bits = s.bits();
        let state = (self.state & !SCConvTime::MS8_244.bits()) | bits;
        let result = self.write_register(Register::CONFIG, state).await;
        self.state = state;
        result
    }

    /// Change the bus voltage conversion time
    #[inline(always)]
    pub async fn set_bvconvtime_mode(&mut self, b: BVConvTime) -> Result<(), I2C::Error> {
        let bits = b.bits();
        let state = (self.state & !BVConvTime::MS8_244.bits()) | bits;
        let result = self.write_register(Register::CONFIG, state).await;
        self.state = state;
        result
    }

    // /// Delivers the unique chip id
    // #[inline(always)]
    // pub async fn did(&mut self) -> Result<u16, I2C::Error> {
    //     let buffer = self.read_reg_u16(Register::DIE_ID).await?;
    //     Ok((u16(buffer[0]) << 8 | u16(buffer[1])) >> 4)
    // }

    // /// Delivers the die revision id
    // #[inline(always)]
    // pub async fn rid(&mut self) -> Result<u16, I2C::Error> {
    //     let result = self.read_reg_u16(Register::MANUFACTURER_ID).await;
    //     match result {
    //         Ok(buffer) => Ok(u16(buffer[1]) & 0b1111),
    //         Err(e) => {
    //             error!("Error reading INA260 Die ID");
    //             Err(e)
    //         }
    //     }
    // }

    /// Delivers the measured raw current in 1.25mA per bit
    #[inline(always)]
    async fn current_raw(&mut self) -> Result<i16, I2C::Error> {
        self.read_reg_i16(Register::CURRENT).await
    }

    /// Delivers the measured current in A
    #[inline(always)]
    pub async fn current(&mut self) -> Result<f32, I2C::Error> {
        Ok(self.current_raw().await? as f32 * 1.25 / 1000.0)
    }

    /// Delivers the measured raw voltage in 1.25mV per bit
    #[inline(always)]
    async fn voltage_raw(&mut self) -> Result<u16, I2C::Error> {
        let mut buffer = [0u8; 2];
        let result = self
            .i2c
            .write_read(self.address, &[Register::VOLTAGE.addr()], &mut buffer)
            .await;
        match result {
            Ok(_) =>{
                let msb = buffer[0];
                let lsb = buffer[1];
                Ok(u16::from_be_bytes([msb, lsb]))
            },
            Err(e) => Err(e),
        }
    }

    /// Delivers the measured current in V
    #[inline(always)]
    pub async fn voltage(&mut self) -> Result<f32, I2C::Error> {
        let result = self.voltage_raw().await;
        match result {
            Ok(raw) => {
                Ok((raw as u32) as f32 * 1.25 / 1000.0) // 1.25mV per bit
            }
            Err(e) => Err(e),
        }
    }

    /// Delivers the measured raw current in 1.25mA per bit
    #[inline(always)]
    async fn power_raw(&mut self) -> Result<u16, I2C::Error> {
        self.read_reg_u16(Register::POWER).await
    }

    /// Delivers the measured power in Watts
    #[inline(always)]
    pub async fn power(&mut self) -> Result<f32, I2C::Error> {
        let raw = self.power_raw().await?;
        Ok(raw as f32 / 100.0) // 10mW per interval
    }
}

use defmt::Format;
#[derive(Debug, Format)]
pub enum Error<E> {
    /// Failed to compensate a raw measurement
    CompensationFailed,
    /// I²C or SPI bus error
    Bus(E),
    /// Failed to parse sensor data
    InvalidData,
}

#[allow(non_camel_case_types)]
#[derive(Copy, Clone)]
pub enum Register {
    // Configuration Register
    CONFIG = 0x00,
    // Contains the value of the current flowing through the shunt resistor
    CURRENT = 0x01,
    // Bus voltage measurement data
    VOLTAGE = 0x02,
    // Contains the value of the calculated power being delivered to the load
    POWER = 0x03,
    // Alert configuration and conversion ready flag
    MASK_ENABLE = 0x06,
    // Contains the limit value to compare to the selected alert function
    ALERT_LIMIT = 0x07,
    // Contains unique manufacturer identification number
    MANUFACTURER_ID = 0xFE,
    // Contains unique die identification number
    DIE_ID = 0xFF,
}

impl Register {
    #[inline(always)]
    pub fn addr(self) -> u8 {
        self as u8
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
/// Averaging Mode
/// Determines the number of samples that are collected and averaged.
pub enum Averaging {
    // No averaging (default)
    AVG1 = 0b0000_0000_0000_0000,
    // 4 times averaging
    AVG4 = 0b0000_0010_0000_0000,
    // 16 times averaging
    AVG16 = 0b0000_0100_0000_0000,
    // 64 times averaging
    AVG64 = 0b0000_0110_0000_0000,
    // 128 times averaging
    AVG128 = 0b0000_1000_0000_0000,
    // 256 times averaging
    AVG256 = 0b0000_1010_0000_0000,
    // 512 times averaging
    AVG512 = 0b0000_1100_0000_0000,
    // 1024 times averaging
    AVG1024 = 0b0000_1110_0000_0000,
}

impl Averaging {
    #[inline(always)]
    pub fn bits(self) -> u16 {
        self as u16
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
/// Bus Voltage Conversion Time
/// Sets the conversion time for the bus voltage measurement
pub enum BVConvTime {
    // Conversion time = 140 µs
    US140 = 0b0000_0000_0000_0000,
    // Conversion time = 204 µs
    US204 = 0b0000_0000_0100_0000,
    // Conversion time = 332 µs
    US332 = 0b0000_0000_1000_0000,
    // Conversion time = 588 µs
    US588 = 0b0000_0000_1100_0000,
    // Conversion time = 1.1 ms (default)
    MS1_1 = 0b0000_0001_0000_0000,
    // Conversion time = 2.116 ms
    MS2_116 = 0b0000_0001_0100_0000,
    // Conversion time = 4.156 ms
    MS4_156 = 0b0000_0001_1000_0000,
    // Conversion time = 8.244 ms
    MS8_244 = 0b0000_0001_1100_0000,
}

impl BVConvTime {
    #[inline(always)]
    pub fn bits(self) -> u16 {
        self as u16
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
/// Shunt Current Conversion Time
/// Sets the conversion time for the shunt current measurement
pub enum SCConvTime {
    // Conversion time = 140 µs
    US140 = 0b0000_0000_0000_0000,
    // Conversion time = 204 µs
    US204 = 0b0000_0000_0000_1000,
    // Conversion time = 332 µs
    US332 = 0b0000_0000_0001_0000,
    // Conversion time = 588 µs
    US588 = 0b0000_0000_0001_1000,
    // Conversion time = 1.1 ms (default)
    MS1_1 = 0b0000_0000_0010_0000,
    // Conversion time = 2.116 ms
    MS2_116 = 0b0000_0000_0010_1000,
    // Conversion time = 4.156 ms
    MS4_156 = 0b0000_0000_0011_0000,
    // Conversion time = 8.244 ms
    MS8_244 = 0b0000_0000_0011_1000,
}

impl SCConvTime {
    #[inline(always)]
    pub fn bits(self) -> u16 {
        self as u16
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
/// Operating Mode
/// Selects continuous, triggered, or power-down mode of operation.
pub enum OperMode {
    // Power-Down (or Shutdown)
    SHUTDOWN = 0b0000_0000_0000_0000,
    // = Shunt Current, Triggered
    SCT = 0b0000_0000_0000_0001,
    // = Shunt Current, Triggered
    BVT = 0b0000_0000_0000_0010,
    // = Shunt Current + Bus Voltage, Triggered
    SCBVT = 0b0000_0000_0000_0011,
    // = Shunt Current, Continuous
    SCC = 0b0000_0000_0000_0101,
    // = Bus Voltage, Continuous
    BVC = 0b0000_0000_0000_0110,
    // = Shunt Current + Bus Voltage, Continuous (default)
    SCBVC = 0b0000_0000_0000_0111,
}

impl OperMode {
    #[inline(always)]
    pub fn bits(self) -> u16 {
        self as u16
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
/// Mask/Enable Register
///
/// The Mask/Enable Register selects the function that is enabled to control the ALERT pin as well as how that pin
/// functions. If multiple functions are enabled, the highest significant bit position Alert Function (D15-D11) takes
/// priority and responds to the Alert Limit Register.
pub enum MaskEnable {
    /// Over Current Limit
    ///
    /// Setting this bit high configures the ALERT pin to be asserted if the current
    /// measurement following a conversion exceeds the value programmed in the Alert
    /// Limit Register.
    OCL = 0b1000_0000_0000_0000,
    /// Under Current Limit
    ///
    /// Setting this bit high configures the ALERT pin to be asserted if the current
    /// measurement following a conversion drops below the value programmed in the
    /// Alert Limit Register.
    UCL = 0b0100_0000_0000_0000,
    /// Bus Voltage Over-Voltage
    ///
    /// Setting this bit high configures the ALERT pin to be asserted if the bus voltage
    /// measurement following a conversion exceeds the value programmed in the Alert
    /// Limit Register.
    BOL = 0b0010_0000_0000_0000,
    /// Bus Voltage Under-Voltage
    ///
    /// Setting this bit high configures the ALERT pin to be asserted if the bus voltage
    /// measurement following a conversion drops below the value programmed in the
    /// Alert Limit Register.
    BUL = 0b0001_0000_0000_0000,
    /// Power Over-Limit
    ///
    /// Setting this bit high configures the ALERT pin to be asserted if the Power
    /// calculation made following a bus voltage measurement exceeds the value
    /// programmed in the Alert Limit Register.
    POL = 0b0000_1000_0000_0000,
    /// Conversion Ready
    ///
    /// Setting this bit high configures the ALERT pin to be asserted when the Conversion
    /// Ready Flag, Bit 3, is asserted indicating that the device is ready for the next
    /// conversion.
    CNVR = 0b0000_0100_0000_0000,
    /// Alert Function Flag
    ///
    /// While only one Alert Function can be monitored at the ALERT pin at a time, the
    /// Conversion Ready can also be enabled to assert the ALERT pin. Reading the Alert
    /// Function Flag following an alert allows the user to determine if the Alert Function
    /// was the source of the Alert.
    ///
    /// When the Alert Latch Enable bit is set to Latch mode, the Alert Function Flag bit
    /// clears only when the Mask/Enable Register is read. When the Alert Latch Enable
    /// bit is set to Transparent mode, the Alert Function Flag bit is cleared following the
    /// next conversion that does not result in an Alert condition.
    AFF = 0b0000_0000_0001_0000,
    /// Conversion Ready
    ///
    /// Although the device can be read at any time, and the data from the last conversion
    /// is available, the Conversion Ready Flag bit is provided to help coordinate one-shot
    /// or triggered conversions. The Conversion Ready Flag bit is set after all
    /// conversions, averaging, and multiplications are complete. Conversion Ready Flag
    /// bit clears under the following conditions:
    ///
    /// 1.) Writing to the Configuration Register (except for Power-Down selection)
    /// 2.) Reading the Mask/Enable Register
    CVRF = 0b0000_0000_0000_1000,
    /// Math Overflow Flag
    ///
    /// This bit is set to '1' if an arithmetic operation resulted in an overflow error. It
    /// indicates that power data may have exceeded the maximum reportable value of
    /// 419.43 W.
    OVF = 0b0000_0000_0000_0100,
    /// Alert Polarity bit
    ///
    /// 1 = Inverted (active-high open collector)
    /// 0 = Normal (active-low open collector) (default)
    APOL = 0b0000_0000_0000_0010,
    /// Alert Latch Enable; configures the latching feature of the ALERT pin and Alert Flag
    /// bits.
    ///
    /// 1 = Latch enabled
    /// 0 = Transparent (default)
    ///
    /// When the Alert Latch Enable bit is set to Transparent mode, the ALERT pin and
    /// Flag bit resets to the idle states when the fault has been cleared. When the Alert
    /// Latch Enable bit is set to Latch mode, the ALERT pin and Alert Flag bit remains
    /// active following a fault until the Mask/Enable Register has been read.
    LEN = 0b0000_0000_0000_0001,
}

impl MaskEnable {
    #[inline(always)]
    pub fn bits(self) -> u16 {
        self as u16
    }
}
