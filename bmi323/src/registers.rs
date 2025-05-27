use embedded_hal_async::i2c::I2c;

pub trait BmiDevice: I2c {
    fn write_register<R: Register>(value: u16) -> Result<(), Self::Error> {
        todo!();
    }

    fn read_register<R: Register>(reg: &R) -> Result<u16, Self::Error> {
        todo!()
    }
}

pub trait Register {
    const ADDRESS: u8;
    const DEFAULT: u8;
}
