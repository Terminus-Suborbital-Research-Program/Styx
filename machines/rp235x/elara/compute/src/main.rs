use std::io::{BufRead, BufReader, Write};
use std::time::{Duration, Instant};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "compute", about = "UART test harness for the satellite compute link")]
struct Cli {
    #[arg(long, help = "Serial device path, for example /dev/serial0 or /dev/ttyAMA0")]
    port: String,

    #[arg(long, default_value_t = satellite::COMPUTE_UART_BAUD, help = "UART baud rate")]
    baud: u32,

    #[arg(long, default_value_t = 1000, help = "Heartbeat period in milliseconds")]
    ping_ms: u64,

    #[arg(long, help = "Send a one-shot ECHO command after opening the link")]
    echo: Option<String>,

    #[arg(long, value_name = "X,Y,Z", help = "Send a one-shot MOTORS command with signed percent values")]
    motors: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let port = serialport::new(&cli.port, cli.baud)
        .timeout(Duration::from_millis(200))
        .open()?;

    let mut writer = port.try_clone()?;
    let reader = BufReader::new(port);

    if let Some(message) = &cli.echo {
        writeln!(writer, "ECHO {}", message)?;
        writer.flush()?;
    }

    if let Some(motors) = &cli.motors {
        let [x, y, z] = parse_motor_triplet(motors)?;
        writeln!(writer, "MOTORS {} {} {}", x, y, z)?;
        writer.flush()?;
    }

    let ping_period = Duration::from_millis(cli.ping_ms.max(50));
    let mut next_ping = Instant::now();
    let mut seq: u32 = 0;

    std::thread::spawn(move || {
        for line in reader.lines() {
            match line {
                Ok(line) if !line.trim().is_empty() => println!("{}", line),
                Ok(_) => {}
                Err(err) => {
                    eprintln!("serial read error: {}", err);
                    break;
                }
            }
        }
    });

    loop {
        if Instant::now() >= next_ping {
            writeln!(writer, "PING {}", seq)?;
            writer.flush()?;
            seq = seq.wrapping_add(1);
            next_ping += ping_period;
        }

        std::thread::sleep(Duration::from_millis(20));
    }
}

fn parse_motor_triplet(value: &str) -> Result<[i8; 3], Box<dyn std::error::Error>> {
    let mut parts = value.split(',').map(str::trim);
    let x: i8 = parts.next().ok_or("missing motor x")?.parse()?;
    let y: i8 = parts.next().ok_or("missing motor y")?.parse()?;
    let z: i8 = parts.next().ok_or("missing motor z")?.parse()?;

    if parts.next().is_some() {
        return Err("expected exactly three motor values".into());
    }

    Ok([x.clamp(-100, 100), y.clamp(-100, 100), z.clamp(-100, 100)])
}