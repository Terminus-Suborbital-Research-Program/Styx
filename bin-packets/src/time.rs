use bincode::{Decode, Encode};
use defmt::Format;
use serde::{Deserialize, Serialize};

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
pub struct Timestamp {
    pub timestamp: u64,
}

#[allow(dead_code)]
impl Timestamp {
    /// Create a new timestamp from a u64
    pub fn new(timestamp: u64) -> Self {
        Self { timestamp }
    }

    /// Get the timestamp from the zero epoch
    pub fn epoch() -> Self {
        Self { timestamp: 0 }
    }

    /// Current timestamp in milliseconds
    pub fn millis(self) -> u64 {
        // Nanos -> millis = / 1_000_000
        self.timestamp / 1_000_000
    }

    /// Current timestamp in seconds
    pub fn seconds(self) -> u64 {
        // Nanos -> seconds = / 1_000_000_000
        self.timestamp / 1_000_000_000
    }

    /// Current timestamp in microseconds
    pub fn micros(self) -> u64 {
        // Nanos -> micros = / 1_000
        self.timestamp / 1_000
    }

    /// Current timestamp in nanoseconds
    pub fn nanos(self) -> u64 {
        // Nanos -> nanos = * 1
        self.timestamp
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::epoch()
    }
}

// Impliment add + sub for Timestamp and DurationMillis
impl core::ops::Add<DurationMillis> for Timestamp {
    type Output = Timestamp;

    fn add(self, rhs: DurationMillis) -> Self::Output {
        Timestamp {
            timestamp: self.timestamp + rhs.duration,
        }
    }
}

impl core::ops::Sub<DurationMillis> for Timestamp {
    type Output = Result<Timestamp, SubtractionUnderflowError>;

    fn sub(self, rhs: DurationMillis) -> Self::Output {
        if self.timestamp < rhs.duration {
            return Err(SubtractionUnderflowError);
        }

        Ok(Timestamp {
            timestamp: self.timestamp - rhs.duration,
        })
    }
}

/// Error for when subtraction would result in an underflow
#[derive(Debug, Clone, Copy, Format)]
pub struct SubtractionUnderflowError;

/// Subtracting two Timestamp results in a DurationMillis
/// We abs this so that we can always get a positive duration
impl core::ops::Sub<Timestamp> for Timestamp {
    type Output = Result<DurationMillis, SubtractionUnderflowError>;

    fn sub(self, rhs: Timestamp) -> Self::Output {
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
        // Nanos -> millis = / 1_000_000
        self.duration / 1_000_000
    }
}

#[cfg(test)]
mod timestamp_tests {
    use super::*;

    #[test]
    fn test_unix_timestamp_millis() {
        let timestamp = Timestamp::new(1000);
        assert_eq!(timestamp.nanos(), 1000);
    }

    #[test]
    fn test_unix_timestamp_millis_epoch() {
        let timestamp = Timestamp::epoch();
        assert_eq!(timestamp.timestamp, 0);
    }

    /// Equality tests
    #[test]
    fn test_unix_timestamp_millis_eq() {
        let timestamp1 = Timestamp::new(1000);
        let timestamp2 = Timestamp::new(1000);
        assert_eq!(timestamp1, timestamp2);
    }

    /// Ordering tests
    #[test]
    fn test_unix_timestamp_millis_ord() {
        let timestamp1 = Timestamp::new(1000);
        let timestamp2 = Timestamp::new(2000);
        assert!(timestamp1 < timestamp2);
    }

    /// Add some durations to a timestamp, make sure it is equal to the expected value
    #[test]
    fn test_unix_timestamp_millis_add() {
        let timestamp = Timestamp::new(1000);
        let duration = DurationMillis::new(1000);
        let new_timestamp = timestamp + duration;
        assert_eq!(new_timestamp.timestamp, 2000);

        // Check that the duration between two timestamps is correct
        let timestamp1 = Timestamp::new(1000);
        let timestamp2 = Timestamp::new(2000);
        let duration = (timestamp2 - timestamp1).expect("Underflow");

        assert_eq!(duration.duration, 1000);
    }

    /// Subtract some durations from a timestamp, make sure it is equal to the expected value
    /// Also test that the duration between two timestamps is correct
    #[test]
    fn test_unix_timestamp_millis_sub() {
        let timestamp1 = Timestamp::new(2000);
        let timestamp2 = Timestamp::new(1000);
        let duration = (timestamp1 - timestamp2).expect("Underflow");
        assert_eq!(duration.duration, 1000);

        let new_timestamp = (timestamp1 - duration).expect("Underflow");
        assert_eq!(new_timestamp.timestamp, 1000);
    }

    /// Subtracting two timestamps should result in a correct duration
    #[test]
    fn test_unix_timestamp_millis_sub_timestamp() {
        let timestamp1 = Timestamp::new(2000);
        let timestamp2 = Timestamp::new(1000);
        let duration = (timestamp1 - timestamp2).expect("Underflow");
        assert_eq!(duration.duration, 1000);
    }

    /// Underflowing should panic in testing mode
    #[test]
    #[should_panic]
    fn test_unix_timestamp_millis_sub_underflow() {
        let timestamp1 = Timestamp::new(1000);
        let timestamp2 = Timestamp::new(2000);
        let _ = (timestamp1 - timestamp2).unwrap();
    }
}
