use bmi323::{Bmi323, AccelConfig, GyroConfig, OutputDataRate, AccelerometerRange, GyroscopeRange};
use linux_embedded_hal::{I2cdev,Delay};
use embedded_hal::i2c::{I2c, Error};
use rppal::i2c::I2c;
use rs_ws281x::{ChannelBuilder, Controller, ControllerBuilder, StripType};
use std::{thread, time::{Duration, Instant}};

const LED_COUNT: i32 = 4;
const I2C_ADDR: u8 = 0x69;   // Your confirmed working address



fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ====================== LED SETUP ======================
    let mut controller: Controller = ControllerBuilder::new()
        .freq(800_000)
        .dma(10)
        .channel(
            0,
            ChannelBuilder::new()
                .pin(18)
                .count(LED_COUNT)
                .strip_type(StripType::Ws2812)
                .brightness(180)
                .build(),
        )
        .build()?;

    // ====================== I2C + BMI323 SETUP (using the official driver) ======================
    let i2c = I2c::<AddressMode::SevenBitAddress>::new("/dev/i2c-0")?;                    // Do NOT call set_slave_address — the driver handles the address
    let delay = Delay;

    let mut imu: Bmi323<_, _> = Bmi323::new_with_i2c(i2c, I2C_ADDR, delay);

    println!("=== BMI323 Relative Compass (using official bmi323 crate) ===");

    // Initialize the sensor (soft reset + basic power-up)
    println!("Initializing BMI323 sensor...");
    imu.init()?;

    // Configure for good performance (you can tweak these values)
    let accel_config = AccelConfig::builder()
        .odr(OutputDataRate::Odr100hz)
        .range(AccelerometerRange::G8)
        .build();
    imu.set_accel_config(accel_config)?;

    let gyro_config = GyroConfig::builder()
        .odr(OutputDataRate::Odr100hz)
        .range(GyroscopeRange::DPS2000)
        .build();
    imu.set_gyro_config(gyro_config)?;

    println!("Sensor configured.");

    // ====================== CALIBRATION WITH GREEN SPINNER ======================
    println!("Calibrating gyro bias — keep the sensor completely STILL for 2 seconds...");
    println!("Watch the green LED spin smoothly around the ring.");

    let cal_start = Instant::now();
    let mut gyro_bias: f32 = 0.0;
    let mut sample_count: u32 = 0;

    while cal_start.elapsed() < Duration::from_secs(2) {
        let gyro_data = imu.read_gyro_data_scaled()?;
        let gz_dps = gyro_data.z;               // library already gives scaled °/s
        gyro_bias += gz_dps;
        sample_count += 1;

        // Green spinning pointer (2 full rotations during calibration)
        let elapsed = cal_start.elapsed().as_secs_f32();
        let progress = elapsed / 2.0;
        let cal_angle = progress * 720.0;
        let led_pos = (cal_angle % 360.0) / 90.0;
        let current_led = (led_pos.floor() as usize) % 4;
        let next_led = (current_led + 1) % 4;
        let frac = led_pos.fract();

        let leds = controller.leds_mut(0);
        for i in 0..LED_COUNT as usize {
            let color = if i == current_led {
                let t = 1.0 - frac;
                let g = (t * 255.0) as u8;
                [0, g, 0, 0]
            } else if i == next_led {
                let t = frac;
                let g = (t * 255.0) as u8;
                [0, g, 0, 0]
            } else {
                [0, 0, 0, 0]
            };
            leds[i] = color;
        }
        controller.render()?;

        thread::sleep(Duration::from_millis(20));
    }
    gyro_bias /= sample_count as f32;
    println!("✅ Calibration complete! Gyro bias: {:.2} dps", gyro_bias);

    println!("Current direction is now NORTH (red pointer starts on LED #0).");
    println!("Rotate the sensor — the red pointer moves smoothly around the blue LEDs.");
    println!("Turn until the red is back on LED #0 to face north again.");

    // ====================== MAIN LOOP (Yaw only) ======================
    let mut yaw: f32 = 0.0;
    let mut last_time = Instant::now();
    let mut last_print = Instant::now();

    loop {
        let gyro_data = imu.read_gyro_data_scaled()?;
        let gz_dps = gyro_data.z;

        // Yaw integration
        let now = Instant::now();
        let dt = (now - last_time).as_secs_f32();
        last_time = now;

        let corrected_gz = gz_dps - gyro_bias;
        if corrected_gz.abs() > 3.0 {
            yaw += corrected_gz * dt;
        }
        yaw = yaw % 360.0;
        if yaw < 0.0 {
            yaw += 360.0;
        }

        // Map yaw to LED position (smooth red ↔ blue cross-fade)
        let led_pos = yaw / 90.0;
        let current_led = (led_pos.floor() as usize) % 4;
        let next_led = (current_led + 1) % 4;
        let frac = led_pos.fract();

        let leds = controller.leds_mut(0);
        for i in 0..LED_COUNT as usize {
            let color: [u8; 4] = if i == current_led {
                let t = 1.0 - frac;
                let r = (t * 255.0) as u8;
                let b = ((1.0 - t) * 255.0) as u8;
                [r, 0, b, 0]
            } else if i == next_led {
                let t = frac;
                let r = (t * 255.0) as u8;
                let b = ((1.0 - t) * 255.0) as u8;
                [r, 0, b, 0]
            } else {
                [0, 0, 255, 0]   // pure blue background
            };
            leds[i] = color;
        }

        controller.render()?;

        // Debug every 300 ms
        if last_print.elapsed() > Duration::from_millis(300) {
            println!("Yaw: {:.1}°", yaw);
            last_print = Instant::now();
        }

        thread::sleep(Duration::from_millis(50));   // smooth 20 Hz
    }
}