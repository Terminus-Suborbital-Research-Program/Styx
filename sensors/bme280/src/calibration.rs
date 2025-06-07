// Copyright Claudio Mattera 2022-2024.
//
// Distributed under the MIT License or the Apache 2.0 License at your option.
// See the accompanying files License-MIT.txt and License-Apache-2.0.txt, or
// online at
// https://opensource.org/licenses/MIT
// https://opensource.org/licenses/Apache-2.0

//! Data types and functions for BME280 sensor calibration

/// First I²C register for reading calibration coefficients
pub const FIRST_REGISTER: u8 = 0x88;
/// Length of first part of calibration coefficients
pub const FIRST_LENGTH: usize = 26;

/// Second I²C register for reading calibration coefficients
pub const SECOND_REGISTER: u8 = 0xe1;
/// Length of second part of calibration coefficients
pub const SECOND_LENGTH: usize = 7;

/// Total length of calibration coefficients
pub const TOTAL_LENGTH: usize = FIRST_LENGTH + SECOND_LENGTH;

/// Calibration coefficients
#[allow(clippy::module_name_repetitions)] // Using a more informative name
#[allow(clippy::struct_field_names)] // Using names from datasheet
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CalibrationData {
    /// First temperature coefficient
    pub dig_t1: u16,
    /// Second temperature coefficient
    pub dig_t2: i16,
    /// Third temperature coefficient
    pub dig_t3: i16,

    /// First pressure coefficient
    pub dig_p1: u16,
    /// Second pressure coefficient
    pub dig_p2: i16,
    /// Third pressure coefficient
    pub dig_p3: i16,
    /// Fourth pressure coefficient
    pub dig_p4: i16,
    /// Fifth pressure coefficient
    pub dig_p5: i16,
    /// Sixth pressure coefficient
    pub dig_p6: i16,
    /// Seventh pressure coefficient
    pub dig_p7: i16,
    /// Eighth pressure coefficient
    pub dig_p8: i16,
    /// Ninth pressure coefficient
    pub dig_p9: i16,

    /// First humidity coefficient
    pub dig_h1: u8,
    /// Second humidity coefficient
    pub dig_h2: i16,
    /// Third humidity coefficient
    pub dig_h3: u8,
    /// Fourth humidity coefficient
    pub dig_h4: i16,
    /// Fifth humidity coefficient
    pub dig_h5: i16,
    /// Sixth humidity coefficient
    pub dig_h6: i8,
}

impl From<&[u8; TOTAL_LENGTH]> for CalibrationData {
    fn from(data: &[u8; TOTAL_LENGTH]) -> Self {
        #[allow(clippy::cast_possible_wrap)] // Using documentation
        let dig_h6 = data[32] as i8;

        Self {
            dig_t1: u16::from_le_bytes([data[0], data[1]]),
            dig_t2: i16::from_le_bytes([data[2], data[3]]),
            dig_t3: i16::from_le_bytes([data[4], data[5]]),

            dig_p1: u16::from_le_bytes([data[6], data[7]]),
            dig_p2: i16::from_le_bytes([data[8], data[9]]),
            dig_p3: i16::from_le_bytes([data[10], data[11]]),
            dig_p4: i16::from_le_bytes([data[12], data[13]]),
            dig_p5: i16::from_le_bytes([data[14], data[15]]),
            dig_p6: i16::from_le_bytes([data[16], data[17]]),
            dig_p7: i16::from_le_bytes([data[18], data[19]]),
            dig_p8: i16::from_le_bytes([data[20], data[21]]),
            dig_p9: i16::from_le_bytes([data[22], data[23]]),

            dig_h1: data[25],
            dig_h2: i16::from_le_bytes([data[26], data[27]]),
            dig_h3: data[28],
            dig_h4: i16::from(data[29]) << 4 | i16::from(data[30]) & 0xf,
            dig_h5: ((i16::from(data[30]) & 0xf0) >> 4) | (i16::from(data[31]) << 4),
            dig_h6,
        }
    }
}

impl CalibrationData {
    /// Compute human-readable temperature from raw temperature
    pub fn compensate_temperature(&self, adc_t: u32) -> i32 {
        #[allow(clippy::cast_possible_wrap)] // Using reference algorithm
        let adc_t = adc_t as i32;

        let var1 = (((adc_t >> 3) - (i32::from(self.dig_t1) << 1)) * i32::from(self.dig_t2)) >> 11;
        let var2 = (((((adc_t >> 4) - i32::from(self.dig_t1))
            * ((adc_t >> 4) - i32::from(self.dig_t1)))
            >> 12)
            * i32::from(self.dig_t3))
            >> 14;

        var1 + var2
    }

    /// Compute human-readable pressure from raw pressure and reference temperature
    pub fn compensate_pressure(&self, adc_p: u32, t_fine: i32) -> u32 {
        let var1 = i64::from(t_fine) - 128_000;
        let var2 = var1 * var1 * i64::from(self.dig_p6);
        let var2 = var2 + ((var1 * i64::from(self.dig_p5)) << 17);
        let var2 = var2 + (i64::from(self.dig_p4) << 35);
        let var1 =
            ((var1 * var1 * i64::from(self.dig_p3)) >> 8) + ((var1 * i64::from(self.dig_p2)) << 12);
        let var1 = ((((1_i64) << 47) + var1) * i64::from(self.dig_p1)) >> 33;

        if var1 == 0 {
            // division by zero
            0
        } else {
            let var4 = 1_048_576 - i64::from(adc_p);
            let var4 = (((var4 << 31) - var2) * 3125) / var1;
            let var1 = (i64::from(self.dig_p9) * (var4 >> 13) * (var4 >> 13)) >> 25;
            let var2 = (i64::from(self.dig_p8) * var4) >> 19;
            let var5 = ((var4 + var1 + var2) >> 8) + (i64::from(self.dig_p7) << 4);

            let p = var5;

            #[allow(clippy::cast_sign_loss)] // Using reference algorithm
            #[allow(clippy::cast_possible_truncation)] // Acceptable truncation
            let pressure = p as u32;

            pressure
        }
    }

    /// Compute human-readable humidity from raw humidity and reference temperature
    pub fn compensate_humidity(&self, adc_h: u16, t_fine: i32) -> u32 {
        let adc_h = i32::from(adc_h);

        let v_x1_u32r: i32 = t_fine - 76_800_i32;
        let v_x1_u32r: i32 = ((((adc_h << 14)
            - (i32::from(self.dig_h4) << 20)
            - (i32::from(self.dig_h5) * v_x1_u32r))
            + (16_384_i32))
            >> 15)
            * (((((((v_x1_u32r * i32::from(self.dig_h6)) >> 10)
                * (((v_x1_u32r * i32::from(self.dig_h3)) >> 11) + (32_768_i32)))
                >> 10)
                + (2_097_152_i32))
                * i32::from(self.dig_h2)
                + 8192_i32)
                >> 14);
        let v_x1_u32r: i32 = v_x1_u32r
            - (((((v_x1_u32r >> 15) * (v_x1_u32r >> 15)) >> 7) * i32::from(self.dig_h1)) >> 4);
        let v_x1_u32r = if v_x1_u32r < 0 { 0 } else { v_x1_u32r };
        let v_x1_u32r = if v_x1_u32r > 419_430_400 {
            419_430_400
        } else {
            v_x1_u32r
        };

        let humidity = v_x1_u32r >> 12;

        #[allow(clippy::cast_sign_loss)] // Using reference algorithm
        let humidity = humidity as u32;

        humidity
    }
}