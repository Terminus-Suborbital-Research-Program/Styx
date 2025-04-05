/// Convert to little-endian byte array
pub fn data_word_from_u32(data: u32) -> [u8; 4] {
    data.to_le_bytes()
}

/// Convert from little-endian byte array
pub fn data_word_to_u32(data: [u8; 4]) -> u32 {
    u32::from_le_bytes(data)
}

#[cfg(test)]
mod tests {
    pub use super::*;

    /// Convert to little-endian byte array. Assertation from table 7-11 and 7-12 of the [dataseheet](https://www.ti.com/lit/ds/symlink/mcf8316c-q1.pdf)
    #[test]
    fn test_data_word_from_u32() {
        let data: u32 = 0x1234ABCD;
        let bytes: [u8; 4] = data_word_from_u32(data);
        assert_eq!(bytes[0], 0xCD);
        assert_eq!(bytes[1], 0xAB);
        assert_eq!(bytes[2], 0x34);
        assert_eq!(bytes[3], 0x12);
    }

    /// Convert from little-endian byte array. Assertation from table 7-11 and 7-12 of the [dataseheet](https://www.ti.com/lit/ds/symlink/mcf8316c-q1.pdf)
    #[test]
    fn test_data_word_to_u32() {
        let bytes: [u8; 4] = [0xCD, 0xAB, 0x34, 0x12];
        let data: u32 = data_word_to_u32(bytes);
        assert_eq!(data, 0x1234ABCD);
    }
}
