use bincode::{Decode, Encode};
use defmt::Format;
use serde::{Deserialize, Serialize};
use fugit::Instant;

/// Used to represent unix timestamp in milliseconds
#[derive(
    Debug,
    Clone,
    Copy,
    Encode,
    Decode,
    Format,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
pub struct UnixTimestampMillis {
    pub timestamp: u64,
}

#[allow(dead_code)]
impl UnixTimestampMillis {
    /// Create a new timestamp from a u64
    pub fn new(timestamp: u64) -> Self {
        Self { timestamp }
    }

    /// Get the timestamp from the zero epoch
    pub fn epoch() -> Self {
        Self { timestamp: 0 }
    }
}

impl Default for UnixTimestampMillis {
    fn default() -> Self {
        Self::epoch()
    }
}

// Impliment add + sub for UnixTimestampMillis and DurationMillis
impl core::ops::Add<DurationMillis> for UnixTimestampMillis {
    type Output = UnixTimestampMillis;

    fn add(self, rhs: DurationMillis) -> Self::Output {
        UnixTimestampMillis {
            timestamp: self.timestamp + rhs.duration,
        }
    }
}

impl core::ops::Sub<DurationMillis> for UnixTimestampMillis {
    type Output = Result<UnixTimestampMillis, SubtractionUnderflowError>;

    fn sub(self, rhs: DurationMillis) -> Self::Output {
        if self.timestamp < rhs.duration {
            return Err(SubtractionUnderflowError);
        }

        Ok(UnixTimestampMillis {
            timestamp: self.timestamp - rhs.duration,
        })
    }
}

/// Error for when subtraction would result in an underflow
#[derive(Debug, Clone, Copy, Format)]
pub struct SubtractionUnderflowError;

/// Subtracting two UnixTimestampMillis results in a DurationMillis
/// We abs this so that we can always get a positive duration
impl core::ops::Sub<UnixTimestampMillis> for UnixTimestampMillis {
    type Output = Result<DurationMillis, SubtractionUnderflowError>;

    fn sub(self, rhs: UnixTimestampMillis) -> Self::Output {
        if self.timestamp < rhs.timestamp {
            return Err(SubtractionUnderflowError);
        }

        Ok(DurationMillis {
            duration: self.timestamp - rhs.timestamp,
        })
    }
}

/// A duration represented in milliseconds
#[derive(Debug, Clone, Copy, Encode, Decode, Format, Serialize, Deserialize)]
pub struct DurationMillis {
    pub duration: u64,
}

impl DurationMillis {
    pub fn new(duration: u64) -> Self {
        Self { duration }
    }

    pub fn millis(self) -> u64 {
        self.duration
    }
}

#[cfg(test)]
mod timestamp_tests {
    use super::*;

    #[test]
    fn test_unix_timestamp_millis() {
        let timestamp = UnixTimestampMillis::new(1000);
        assert_eq!(timestamp.timestamp, 1000);
    }

    #[test]
    fn test_unix_timestamp_millis_epoch() {
        let timestamp = UnixTimestampMillis::epoch();
        assert_eq!(timestamp.timestamp, 0);
    }

    /// Equality tests
    #[test]
    fn test_unix_timestamp_millis_eq() {
        let timestamp1 = UnixTimestampMillis::new(1000);
        let timestamp2 = UnixTimestampMillis::new(1000);
        assert_eq!(timestamp1, timestamp2);
    }

    /// Ordering tests
    #[test]
    fn test_unix_timestamp_millis_ord() {
        let timestamp1 = UnixTimestampMillis::new(1000);
        let timestamp2 = UnixTimestampMillis::new(2000);
        assert!(timestamp1 < timestamp2);
    }

    /// Add some durations to a timestamp, make sure it is equal to the expected value
    #[test]
    fn test_unix_timestamp_millis_add() {
        let timestamp = UnixTimestampMillis::new(1000);
        let duration = DurationMillis::new(1000);
        let new_timestamp = timestamp + duration;
        assert_eq!(new_timestamp.timestamp, 2000);

        // Check that the duration between two timestamps is correct
        let timestamp1 = UnixTimestampMillis::new(1000);
        let timestamp2 = UnixTimestampMillis::new(2000);
        let duration = (timestamp2 - timestamp1).expect("Underflow");

        assert_eq!(duration.duration, 1000);
    }

    /// Subtract some durations from a timestamp, make sure it is equal to the expected value
    /// Also test that the duration between two timestamps is correct
    #[test]
    fn test_unix_timestamp_millis_sub() {
        let timestamp1 = UnixTimestampMillis::new(2000);
        let timestamp2 = UnixTimestampMillis::new(1000);
        let duration = (timestamp1 - timestamp2).expect("Underflow");
        assert_eq!(duration.duration, 1000);

        let new_timestamp = (timestamp1 - duration).expect("Underflow");
        assert_eq!(new_timestamp.timestamp, 1000);
    }

    /// Subtracting two timestamps should result in a correct duration
    #[test]
    fn test_unix_timestamp_millis_sub_timestamp() {
        let timestamp1 = UnixTimestampMillis::new(2000);
        let timestamp2 = UnixTimestampMillis::new(1000);
        let duration = (timestamp1 - timestamp2).expect("Underflow");
        assert_eq!(duration.duration, 1000);
    }

    /// Underflowing should panic in testing mode
    #[test]
    #[should_panic]
    fn test_unix_timestamp_millis_sub_underflow() {
        let timestamp1 = UnixTimestampMillis::new(1000);
        let timestamp2 = UnixTimestampMillis::new(2000);
        let _ = (timestamp1 - timestamp2).unwrap();
    }
}


const BUFFER_LENGTH: usize = 10;
// Sensor Data Structs
#[derive(Default,Debug,Clone,Copy,Encode,Decode,Serialize,Deserialize)]
pub struct PowerData{
    time_stamp: u64,
    power: (i8, u32),
}
#[derive(Default,Debug,Clone,Copy,Encode,Decode,Serialize,Deserialize)]
pub struct CurrentData{
    time_stamp: u64,
    current: (i8, u32),
}
#[derive(Default,Debug,Clone,Copy,Encode,Decode,Serialize,Deserialize)]
pub struct VoltageData{
    time_stamp: u64,
    voltage: (u8, u32),
}

#[derive(Default,Debug,Clone,Copy,Encode,Decode,Serialize,Deserialize)]
pub struct MasterData{
    power_1: StaticBuffer<PowerData>,
    power_2: StaticBuffer<PowerData>,
    power_3: StaticBuffer<PowerData>,
    current_1: StaticBuffer<PowerData>,
    current_2: StaticBuffer<PowerData>,
    current_3: StaticBuffer<PowerData>,
    voltage_1: StaticBuffer<PowerData>,
    voltage_2: StaticBuffer<PowerData>,
    voltage_3: StaticBuffer<PowerData>,
}

#[derive(Default,Debug,Clone,Copy,Encode,Decode,Serialize,Deserialize)]
pub struct StaticBuffer<T> {
    buffer: [T; BUFFER_LENGTH],
    index: usize,
}
impl<T: Default + core::marker::Copy> StaticBuffer<T> {
    pub fn new() -> Self {
        Self {
            buffer: [T::default(); BUFFER_LENGTH],
            index: 0,
        }
    }
    pub fn push(&mut self, data: T) {
        if self.index == BUFFER_LENGTH - 1 {
            // Shift all elements to the left, removing the first item
            for i in 0..BUFFER_LENGTH - 1 {
                self.buffer[i] = self.buffer[i + 1].clone();
            }
            self.index = BUFFER_LENGTH - 1;
            // Add the new data to the last position
            self.buffer[self.index] = data;
        } else {
            // Add the new data at the current index
            self.buffer[self.index] = data;
            self.index += 1;
        }
    }
    /// Removes and returns the last element in the buffer if it exists.
    /// 
    /// If the buffer is empty (index is 0), it returns `None`.
    /// Otherwise, it decreases the index by 1 and returns the element
    /// at the previous index.
    pub fn pop(&mut self) -> Option<T> {
        if self.index == 0 {
            None
        } else {
            let result = Some(self.buffer[self.index - 1].clone());
            self.index -= 1;
            return result;
        }
    }
}