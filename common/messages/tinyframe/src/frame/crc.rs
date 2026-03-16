pub fn crc16_ccitt_false(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::crc16_ccitt_false;

    #[test]
    fn test_empty() {
        // Empty slice: returns initial value
        assert_eq!(crc16_ccitt_false(&[]), 0xFFFF);
    }

    #[test]
    fn test_123456789() {
        // Standard test vector for CRC-16-CCITT-FALSE
        // "123456789" → 0x29B1
        let data = b"123456789";
        assert_eq!(crc16_ccitt_false(data), 0x29B1);
    }
}
