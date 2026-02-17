#![warn(missing_docs, redundant_imports, redundant_semicolons)]

use clap::ValueEnum;


#[derive(ValueEnum, Clone, Debug)]
#[value(rename_all = "kebab-case")]
pub enum Tests {
    AllTests,
    I2cTest,
    UartTest,
}

#[derive(ValueEnum, Clone, Debug)]
#[value(rename_all = "kebab-case")]
pub enum Protocol {
    Usb,
    Bluetooth,
    IP,
}
