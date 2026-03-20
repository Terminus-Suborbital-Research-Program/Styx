#![warn(missing_docs)]

use crate::{app::power_switch, device_constants::pins::JupiterI2c};

pub async fn power_switch(mut ctx: power_switch::Context<'_>) {
    let mut i2c_buf:&mut [u8] ;
    while !ctx.local.jupiter_i2c.rx_fifo_empty() {
        ctx.local.jupiter_i2c.read(i2c_buf);
    }
}