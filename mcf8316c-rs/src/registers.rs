use crate::ControlWord;

mod sealed {
    pub trait Sealed {}
}

pub trait ValidAddress: sealed::Sealed {}

pub trait Register: From<u32> + Into<u32> + Copy + Clone + Default + ValidAddress {
    /// The register's 12-bit address
    const ADDRESS: u8;

    /// An set of bytes for a I2C write transaction
    fn i2c_write_bytes(&self) -> [u8; 7] {
        let data_word: u32 = (*self).into();
        let control_word: [u8; 3] = ControlWord::new_write(Self::ADDRESS).to_bytes();

        let data_bytes: [u8; 4] = data_word.to_le_bytes();

        [
            control_word[0],
            control_word[1],
            control_word[2],
            data_bytes[0],
            data_bytes[1],
            data_bytes[2],
            data_bytes[3],
        ]
    }

    /// An set of bytes for a I2C read transaction
    fn i2c_read_bytes(&self) -> [u8; 3] {
        ControlWord::new_read(Self::ADDRESS).to_bytes()
    }

    /// From a data word
    fn from_data_word(data: &[u8; 4]) -> Self {
        let data_word: u32 = u32::from_le_bytes(*data);
        Self::from(data_word)
    }
}
