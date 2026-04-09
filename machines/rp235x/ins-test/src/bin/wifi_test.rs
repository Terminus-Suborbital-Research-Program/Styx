#![no_std]
#![no_main]

use heapless::String;
use cyw43::{JoinOptions, aligned_bytes};
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use defmt::{info, unwrap, warn};
use embassy_executor::Spawner;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{Config, DhcpConfig, StackResources};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::{bind_interrupts, dma};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

const WIFI_NETWORK: &str = "The Aether";
const WIFI_PASSWORD: &str = "ClearSky$";
const WIFI_HOSTNAME: &str = "ins-test";
const UDP_PORT: u16 = 4242;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("bringing up Pico 2 W wifi test");

    let p = embassy_rp::init(Default::default());

    let fw = aligned_bytes!("../../cyw43-firmware/43439A0.bin");
    let clm = aligned_bytes!("../../cyw43-firmware/43439A0_clm.bin");
    let nvram = aligned_bytes!("../../cyw43-firmware/nvram_rp2040.bin");

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        RM2_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        dma::Channel::new(p.DMA_CH0, Irqs),
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;
    spawner.spawn(unwrap!(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;
    control.gpio_set(0, false).await;

    let mut hostname = String::<32>::new();
    unwrap!(hostname.push_str(WIFI_HOSTNAME));
    let mut dhcp_config = DhcpConfig::default();
    dhcp_config.hostname = Some(hostname);
    let config = Config::dhcpv4(dhcp_config);

    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, net_runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        0x494E_532D_5445_5354,
    );
    spawner.spawn(unwrap!(net_task(net_runner)));

    if WIFI_NETWORK.is_empty() {
        warn!("set WIFI_NETWORK and WIFI_PASSWORD in wifi_test.rs to join an AP");

        loop {
            control.gpio_set(0, true).await;
            Timer::after(Duration::from_millis(200)).await;
            control.gpio_set(0, false).await;
            Timer::after(Duration::from_secs(2)).await;
        }
    }

    loop {
        match control
            .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
            .await
        {
            Ok(()) => break,
            Err(_) => {
                warn!("join failed");
                Timer::after(Duration::from_secs(1)).await;
            }
        }
    }

    info!("wifi association complete");
    info!("waiting for DHCP hostname registration: {}", WIFI_HOSTNAME);

    stack.wait_link_up().await;
    stack.wait_config_up().await;

    if let Some(config) = stack.config_v4() {
        info!("dhcp address: {:?}", config.address.address());
    }

    let mut rx_meta = [PacketMetadata::EMPTY; 8];
    let mut tx_meta = [PacketMetadata::EMPTY; 8];
    let mut rx_buffer = [0u8; 2048];
    let mut tx_buffer = [0u8; 2048];
    let mut packet_buffer = [0u8; 1024];

    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );
    unwrap!(socket.bind(UDP_PORT));
    info!("udp echo listening on port {}", UDP_PORT);

    loop {
        let (len, remote) = match socket.recv_from(&mut packet_buffer).await {
            Ok(packet) => packet,
            Err(_) => {
                warn!("udp receive failed");
                Timer::after(Duration::from_millis(250)).await;
                continue;
            }
        };

        info!("udp packet: {} bytes from {:?}", len, remote);
        control.gpio_set(0, true).await;

        if let Err(_) = socket.send_to(&packet_buffer[..len], remote).await {
            warn!("udp echo failed");
        } else {
            info!("udp echo sent: {} bytes", len);
        }

        Timer::after(Duration::from_millis(50)).await;
        control.gpio_set(0, false).await;
    }
}
