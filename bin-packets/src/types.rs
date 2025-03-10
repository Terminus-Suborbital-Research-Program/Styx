use bincode::{Decode, Encode};
use defmt::Format;

/// Used to represent unix timestamp in milliseconds
#[derive(Debug, Clone, Copy, Encode, Decode, Format, PartialEq, Eq, PartialOrd, Ord)]
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
#[derive(Debug, Clone, Copy, Encode, Decode, Format)]
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