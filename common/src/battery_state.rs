
pub enum BatteryState {
    LatchOff,
    LatchOn,
    Neutral,
}


impl From<u8> for BatteryState {
    fn from(value: u8) -> Self {
        match value {
            0 => BatteryState::Neutral,

            1 => BatteryState::LatchOn,

            2 => BatteryState::LatchOff,

            _ => BatteryState::Neutral,
        }
    }
}

impl From<BatteryState> for u8 {
    fn from(value: BatteryState) -> u8 {
        match value {
            BatteryState::Neutral => 0,

            BatteryState::LatchOn => 1,

            BatteryState::LatchOff => 2,
        }
    }
}