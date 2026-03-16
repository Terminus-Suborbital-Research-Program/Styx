use crate::device_constants::servos::RelayServo;
use crate::Mono;
use defmt::{error, info};
use fugit::{Instant, MicrosDurationU64};
use rtic_monotonics::Monotonic;

static RELAY_DEPLOY_TIME: u64 = 10000; //milliseconds

static RELAY_ANGLE_OPEN: u16 = 30;
static RELAY_ANGLE_CLOSE: u16 = 0;

pub static FLUTTER_START_TIME: u64 = 10000; // milliseconds
pub static FLUTTER_COUNT: u8 = 1; // (Open&Close desired times * 2)
pub static SERVO_DISABLE_DELAY: u64 = 2000; // milliseconds
static RELAY_FLUTTER_TIME: u64 = 1000; //milliseconds

// RELAY SERVO STATUS is not generic to SERVO, this is to control flutter sequence, i.e only used here
pub enum RelayServoStatus {
    Open,
    Close,
    Error,
}

pub struct Modes {}

impl Modes {
    async fn relay_servos_actuate(servo: &mut RelayServo) {
        servo.set_angle(RELAY_ANGLE_OPEN);
    }

    pub async fn relay_eject_servo_sequence(
        now: Instant<u64, 1, 1000000>,
        servo: &mut RelayServo,
    ) -> bool {
        // Enable Relay Mosfet -> Meant to lock servos
        servo.enable();
        // Wait until T+Relay Deployment Time
        let deploy_millis = MicrosDurationU64::millis(RELAY_DEPLOY_TIME);
        let tplus_relay_deploy = now + deploy_millis;
        Mono::delay_until(tplus_relay_deploy).await;
        match Mono::timeout_at(tplus_relay_deploy, Self::relay_servos_actuate(servo)).await {
            Ok(_) => {
                info!("Relay Deployed");
                true
            }
            Err(_) => {
                error!("Relay Failed to Deploy");
                false
            }
        }
    }

    pub async fn relay_flutter_sequence(
        now: Instant<u64, 1, 1000000>,
        status: RelayServoStatus,
        servo: &mut RelayServo,
    ) -> RelayServoStatus {
        // Wait until FLUTTER_TIME_NEXT
        let flutter_millis = MicrosDurationU64::millis(RELAY_FLUTTER_TIME);
        let tplus_relay_flutter = now + flutter_millis;
        Mono::delay_until(tplus_relay_flutter).await;
        match status {
            RelayServoStatus::Close => {
                match Mono::timeout_at(tplus_relay_flutter, Self::relay_servos_flutter_open(servo))
                    .await
                {
                    Ok(_) => {
                        info!("Relay FLUTTER OPENED");
                        RelayServoStatus::Open
                    }
                    Err(_) => {
                        error!("Relay FLUTTER ISSUE");
                        RelayServoStatus::Error
                    }
                }
            }
            RelayServoStatus::Open => {
                match Mono::timeout_at(tplus_relay_flutter, Self::relay_servos_flutter_close(servo))
                    .await
                {
                    Ok(_) => {
                        info!("Relay FLUTTER CLOSED");
                        RelayServoStatus::Close
                    }
                    Err(_) => {
                        error!("Relay FLUTTER ISSUE");
                        RelayServoStatus::Error
                    }
                }
            }
            RelayServoStatus::Error => {
                match Mono::timeout_at(tplus_relay_flutter, Self::relay_servos_flutter_open(servo))
                    .await
                {
                    Ok(_) => {
                        info!("Relay FLUTTER CLOSE");
                        RelayServoStatus::Error
                    }
                    Err(_) => {
                        error!("Relay FLUTTER ISSUE");
                        RelayServoStatus::Error
                    }
                }
            }
        }
    }
    async fn relay_servos_flutter_close(servo: &mut RelayServo) {
        servo.set_angle(RELAY_ANGLE_CLOSE);
    }
    async fn relay_servos_flutter_open(servo: &mut RelayServo) {
        servo.set_angle(RELAY_ANGLE_OPEN);
    }
}
