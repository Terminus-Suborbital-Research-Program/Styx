#![warn(missing_docs)]

use crate::{app::power_switch, device_constants::pins::JupiterI2c};

pub async fn power_switch(mut ctx: power_switch::Context<'_>) {
    let mut i2c_buf= [0u8; 40]; ;
    //i2c_buf = &mut [bin_packets::i2c::I2CPacket::PowerLatch as u8; 1];
    
    //ctx.local.jupiter_i2c.write(i2c_buf);
    while !ctx.local.jupiter_i2c.rx_fifo_empty() {
        ctx.local.jupiter_i2c.read(&mut i2c_buf);
    }
}