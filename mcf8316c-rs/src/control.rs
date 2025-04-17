use defmt::Format;

/// Specifies a read or a write transaction
#[derive(Debug, Clone, Copy, Format, PartialEq, Eq)]
#[repr(u8)]
pub enum Transaction {
    /// Write transaction
    Write = 0x00,
    /// Read transaction
    Read = 0x01,
}

impl From<Transaction> for u8 {
    fn from(transaction: Transaction) -> Self {
        transaction as u8
    }
}

/// Data length of read/write transaction
#[derive(Debug, Clone, Copy, Format, PartialEq, Eq)]
#[repr(u8)]
pub enum DataLength {
    Bits16 = 0b00,
    Bits32 = 0b01,
    Bits64 = 0b10,
}

impl From<DataLength> for u8 {
    fn from(data_length: DataLength) -> Self {
        data_length as u8
    }
}

/// Used to specify a read/write transaction on the ESC
#[derive(Debug, Clone, Copy, Format)]
pub struct ControlWord {
    /// Type of transaction
    transaction: Transaction,
    /// If CRC is enabled
    crc: bool,
    /// Data length of transaction
    data_length: DataLength,
    /// Memory page,
    memory_page: u8,
    /// Memory section
    memory_section: u8,
    /// Address of the transaction
    address: u16, // Only twelve bits used
}

impl ControlWord {
    /// Creates a new control word
    pub fn new(transaction: Transaction, crc: bool, data_length: DataLength, address: u16) -> Self {
        ControlWord {
            transaction,
            crc,
            data_length,
            memory_page: 0,
            memory_section: 0,
            address,
        }
    }

    /// Easy 32-bit write transaction 'address', no CRC
    pub fn new_write(address: u16) -> Self {
        ControlWord::new(Transaction::Write, false, DataLength::Bits32, address)
    }

    /// Easy 32-bit read transaction 'address', no CRC
    pub fn new_read(address: u16) -> Self {
        ControlWord::new(Transaction::Read, false, DataLength::Bits32, address)
    }

    /// Turn into the 24-bit control word and then to the three-byte array
    pub fn to_bytes(self) -> [u8; 3] {
        let mut control_word: u32 = 0;

        // Byte 23 is the transaction type
        control_word |= (self.transaction as u32) << 23;
        // Byte 22 is the CRC
        control_word |= (self.crc as u32) << 22;
        // 21-20 are the data length
        control_word |= (self.data_length as u32) << 20;
        // 19-16 are the memory page
        control_word |= (self.memory_page as u32) << 16;
        // 15-12 are the memory section
        control_word |= (self.memory_section as u32) << 12;
        // 11-0 are the address
        control_word |= (self.address as u32) << 0;

        let bytes: [u8; 3] = [
            (control_word >> 16) as u8,
            (control_word >> 8) as u8,
            (control_word & 0xFF) as u8,
        ];

        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Example word, given by table 7-11 in the [dataseheet](https://www.ti.com/lit/ds/symlink/mcf8316c-q1.pdf)
    #[test]
    fn test_32_bit_write() {
        // Address 0x80
        let control_word = ControlWord::new_write(0x80);
        let bytes = control_word.to_bytes();

        assert_eq!(bytes[0], 0x10);
        assert_eq!(bytes[1], 0x00);
        assert_eq!(bytes[2], 0x80);
    }

    /// Example word, given by table 7-13 in the [dataseheet](https://www.ti.com/lit/ds/symlink/mcf8316c-q1.pdf)
    #[test]
    fn test_32_bit_read() {
        // Address 0x80
        let control_word = ControlWord::new_read(0x80);
        let bytes = control_word.to_bytes();

        assert_eq!(bytes[0], 0x90);
        assert_eq!(bytes[1], 0x00);
        assert_eq!(bytes[2], 0x80);
    }
}
