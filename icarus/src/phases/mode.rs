use fugit::{Instant, MicrosDurationU64, MillisDuration, MillisDurationU64};
use rtic_monotonics::Monotonic;
use crate::{Mono};
use defmt::{info, error};
use crate::actuators::servo::Servo;
use crate::device_constants::servos::{FlapServo, RelayServo};

// ICARUS Mission Constants
static FLAP_DEPLOY_TIME: u64 = 5000; //milliseconds
static RELAY_DEPLOY_TIME: u64 = 10000; //milliseconds

pub struct Modes{

}

impl Modes{
    async fn flap_servos_actuate(mut servo: &mut FlapServo){
        servo.deg_90();
    }
    async fn relay_servos_actuate(mut servo: &mut RelayServo){
        servo.deg_90();
    }
    pub async fn open_flaps_sequence(mut now: Instant<u64, 1, 1000000>, mut servo: &mut FlapServo)->bool{
        // Enable Flap Mosfet -> Meant to lock servos
        servo.enable();
        // Wait until T+FLAP_DEPLOY_TIME
        let deploy_millis = MicrosDurationU64::millis(FLAP_DEPLOY_TIME);
        let tplus_flap_deploy = now + deploy_millis;
        Mono::delay_until(tplus_flap_deploy).await;
        match Mono::timeout_at(tplus_flap_deploy, Self::flap_servos_actuate(servo)).await{
            Ok(_)=>{
                info!("Servos Open");
                return true
            }
            Err(_)=>{
                error!("Error Opening Servos");
                return false
            }
        }
    }
    pub async fn eject_servo_sequence(mut now: Instant<u64, 1, 1000000>, mut servo: &mut RelayServo)->bool{
        // Enable Relay Mosfet -> Meant to lock servos
        servo.enable();
        // Wait until T+Relay Deployment Time
        let deploy_millis = MicrosDurationU64::millis(RELAY_DEPLOY_TIME);
        let tplus_relay_deploy = now + deploy_millis;
        Mono::delay_until(tplus_relay_deploy).await;
        match Mono::timeout_at(tplus_relay_deploy, Self::relay_servos_actuate(servo)).await{
            Ok(_)=>{
                info!("Relay Deployed");
                return true
            }
            Err(_)=>{
                error!("Relay Failed to Deploy");
                return false
            }
        }
    }
    
}
