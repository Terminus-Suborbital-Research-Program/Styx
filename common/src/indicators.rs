use bincode::{BorrowDecode, Decode, Encode};
use embedded_hal::digital::PinState;

/// The state of the vehicle's indicators to JUPITER
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndicatorStates {
    gse1: PinState,
    gse2: PinState,
    te_ra: PinState,
    te_rb: PinState,
    te1: PinState,
    te2: PinState,
    te3: PinState,
}

impl IndicatorStates {
    /// GSE-1 value
    pub fn gse1(&self) -> PinState {
        self.gse1
    }

    /// GSE-2 value
    pub fn gse2(&self) -> PinState {
        self.gse2
    }

    /// TE-RA value
    pub fn te_ra(&self) -> PinState {
        self.te_ra
    }

    /// TE-RB value
    pub fn te_rb(&self) -> PinState {
        self.te_rb
    }

    /// TE-1 value
    pub fn te1(&self) -> PinState {
        self.te1
    }

    /// TE-2 value
    pub fn te2(&self) -> PinState {
        self.te2
    }

    /// TE-3 value
    pub fn te3(&self) -> PinState {
        self.te3
    }

    /// Encodes to a u8 to be sent across a bus as a byte
    pub fn encode_i2c(&self) -> u8 {
        self.into()
    }

    /// No pins are high - not default because we don't want this being used nilly-willy, but it's good to have
    /// on occasion to prevent panic
    pub fn none() -> Self {
        IndicatorBuilder::new()
            .gse1(false)
            .gse2(false)
            .te_ra(false)
            .te_rb(false)
            .te1(false)
            .te2(false)
            .te3(false)
            .build()
    }
}

/// Malformed indicator error
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct MalformedIndicatorError {
    /// The value that was malformed
    pub value: u8,
}

/// Turn the indicator states into a u8. Mostly for i2c, see the encode_i2c and decode_i2c functions. If the high bit is set, then this is malformed!
impl TryFrom<u8> for IndicatorStates {
    type Error = MalformedIndicatorError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let gse1 = (value >> 0) & 0b1 != 0;
        let gse2 = (value >> 1) & 0b1 != 0;
        let te_ra = (value >> 2) & 0b1 != 0;
        let te_rb = (value >> 3) & 0b1 != 0;
        let te1 = (value >> 4) & 0b1 != 0;
        let te2 = (value >> 5) & 0b1 != 0;
        let te3 = (value >> 6) & 0b1 != 0;

        if value & 0b10000000 != 0 {
            Err(MalformedIndicatorError { value })
        } else {
            Ok(IndicatorBuilder::new()
                .gse1(gse1)
                .gse2(gse2)
                .te_ra(te_ra)
                .te_rb(te_rb)
                .te1(te1)
                .te2(te2)
                .te3(te3)
                .build())
        }
    }
}

/// Turn the indicator states into a u8. Mostly for i2c, see the encode_i2c and decode_i2c functions
impl From<IndicatorStates> for u8 {
    fn from(value: IndicatorStates) -> Self {
        let mut result = 0;
        result |= (value.gse1 as u8) << 0;
        result |= (value.gse2 as u8) << 1;
        result |= (value.te_ra as u8) << 2;
        result |= (value.te_rb as u8) << 3;
        result |= (value.te1 as u8) << 4;
        result |= (value.te2 as u8) << 5;
        result |= (value.te3 as u8) << 6;
        result
    }
}

impl From<&IndicatorStates> for u8 {
    fn from(value: &IndicatorStates) -> Self {
        IndicatorStates::into(*value)
    }
}

/// A pin that hasn't been set as having high or low
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, Default)]
pub struct Unset {}

/// A pin that has been set to a pin state
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy)]
pub struct Set {
    state: PinState,
}

impl Set {
    pub(crate) fn new<S: Into<PinState>>(from: S) -> Self {
        Set { state: from.into() }
    }
}

impl From<PinState> for Set {
    fn from(value: PinState) -> Self {
        Set::new(value)
    }
}

impl Into<PinState> for Set {
    fn into(self) -> PinState {
        self.state
    }
}

/// A builder for the IndicatorStates struct.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct IndicatorBuilder<GSE1, GSE2, TERA, TERB, TE1, TE2, TE3> {
    gse1: GSE1,
    gse2: GSE2,
    te_ra: TERA,
    te_rb: TERB,
    te1: TE1,
    te2: TE2,
    te3: TE3,
}

impl IndicatorBuilder<Set, Set, Set, Set, Set, Set, Set> {
    pub fn build(self) -> IndicatorStates {
        IndicatorStates {
            gse1: self.gse1.into(),
            gse2: self.gse2.into(),
            te_ra: self.te_ra.into(),
            te_rb: self.te_rb.into(),
            te1: self.te1.into(),
            te2: self.te2.into(),
            te3: self.te3.into(),
        }
    }
}

impl IndicatorBuilder<Unset, Unset, Unset, Unset, Unset, Unset, Unset> {
    pub fn new() -> Self {
        IndicatorBuilder {
            gse1: Unset::default(),
            gse2: Unset::default(),
            te_ra: Unset::default(),
            te_rb: Unset::default(),
            te1: Unset::default(),
            te2: Unset::default(),
            te3: Unset::default(),
        }
    }
}

impl<A, B, C, D, E, F> IndicatorBuilder<Unset, A, B, C, D, E, F> {
    /// Set the state of GSE-1
    pub fn gse1<State: Into<PinState>>(
        self,
        state: State,
    ) -> IndicatorBuilder<Set, A, B, C, D, E, F> {
        IndicatorBuilder {
            gse1: Set::new(state),
            gse2: self.gse2,
            te_ra: self.te_ra,
            te_rb: self.te_rb,
            te1: self.te1,
            te2: self.te2,
            te3: self.te3,
        }
    }
}

impl<A, B, C, D, E, F> IndicatorBuilder<A, Unset, B, C, D, E, F> {
    /// Set the state of GSE-2
    pub fn gse2<State: Into<PinState>>(
        self,
        state: State,
    ) -> IndicatorBuilder<A, Set, B, C, D, E, F> {
        IndicatorBuilder {
            gse1: self.gse1,
            gse2: Set::new(state),
            te_ra: self.te_ra,
            te_rb: self.te_rb,
            te1: self.te1,
            te2: self.te2,
            te3: self.te3,
        }
    }
}

impl<A, B, C, D, E, F> IndicatorBuilder<A, B, Unset, C, D, E, F> {
    /// Set the state of TE-RA
    pub fn te_ra<State: Into<PinState>>(
        self,
        state: State,
    ) -> IndicatorBuilder<A, B, Set, C, D, E, F> {
        IndicatorBuilder {
            gse1: self.gse1,
            gse2: self.gse2,
            te_ra: Set::new(state),
            te_rb: self.te_rb,
            te1: self.te1,
            te2: self.te2,
            te3: self.te3,
        }
    }
}

impl<A, B, C, D, E, F> IndicatorBuilder<A, B, C, Unset, D, E, F> {
    /// Set the state of TE-RB
    pub fn te_rb<State: Into<PinState>>(
        self,
        state: State,
    ) -> IndicatorBuilder<A, B, C, Set, D, E, F> {
        IndicatorBuilder {
            gse1: self.gse1,
            gse2: self.gse2,
            te_ra: self.te_ra,
            te_rb: Set::new(state),
            te1: self.te1,
            te2: self.te2,
            te3: self.te3,
        }
    }
}

impl<A, B, C, D, E, F> IndicatorBuilder<A, B, C, D, Unset, E, F> {
    /// Set the state of TE-1
    pub fn te1<State: Into<PinState>>(
        self,
        state: State,
    ) -> IndicatorBuilder<A, B, C, D, Set, E, F> {
        IndicatorBuilder {
            gse1: self.gse1,
            gse2: self.gse2,
            te_ra: self.te_ra,
            te_rb: self.te_rb,
            te1: Set::new(state),
            te2: self.te2,
            te3: self.te3,
        }
    }
}

impl<A, B, C, D, E, F> IndicatorBuilder<A, B, C, D, E, Unset, F> {
    /// Set the state of TE-2
    pub fn te2<State: Into<PinState>>(
        self,
        state: State,
    ) -> IndicatorBuilder<A, B, C, D, E, Set, F> {
        IndicatorBuilder {
            gse1: self.gse1,
            gse2: self.gse2,
            te_ra: self.te_ra,
            te_rb: self.te_rb,
            te1: self.te1,
            te2: Set::new(state),
            te3: self.te3,
        }
    }
}

impl<A, B, C, D, E, F> IndicatorBuilder<A, B, C, D, E, F, Unset> {
    /// Set the state of TE-3
    pub fn te3<State: Into<PinState>>(
        self,
        state: State,
    ) -> IndicatorBuilder<A, B, C, D, E, F, Set> {
        IndicatorBuilder {
            gse1: self.gse1,
            gse2: self.gse2,
            te_ra: self.te_ra,
            te_rb: self.te_rb,
            te1: self.te1,
            te2: self.te2,
            te3: Set::new(state),
        }
    }
}

/// A container to allow for decoding and encoding [`IndicatorStates`] with bincode
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Encode, Decode)]
struct StatesContainer {
    gse1: bool,
    gse2: bool,
    te_ra: bool,
    te_rb: bool,
    te1: bool,
    te2: bool,
    te3: bool,
}

impl From<IndicatorStates> for StatesContainer {
    fn from(value: IndicatorStates) -> Self {
        StatesContainer {
            gse1: value.gse1.into(),
            gse2: value.gse2.into(),
            te_ra: value.te_ra.into(),
            te_rb: value.te_rb.into(),
            te1: value.te1.into(),
            te2: value.te2.into(),
            te3: value.te3.into(),
        }
    }
}

impl Into<IndicatorStates> for StatesContainer {
    fn into(self) -> IndicatorStates {
        IndicatorStates {
            gse1: self.gse1.into(),
            gse2: self.gse2.into(),
            te_ra: self.te_ra.into(),
            te_rb: self.te_rb.into(),
            te1: self.te1.into(),
            te2: self.te2.into(),
            te3: self.te3.into(),
        }
    }
}

/// Encode with bincode
impl Encode for IndicatorStates {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        let container: StatesContainer = (*self).into();

        container.encode(encoder)
    }
}

/// Decode with bincode
impl<Context> Decode<Context> for IndicatorStates {
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let container: StatesContainer = StatesContainer::decode(decoder)?;

        Ok(container.into())
    }
}

/// Borrowed decode with bincode
impl<'de, Context> BorrowDecode<'de, Context> for IndicatorStates {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let container: StatesContainer = StatesContainer::borrow_decode(decoder)?;
        Ok(container.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Encode to a u8, gse-2 high and te3 high should be 0b01000010
    fn test_encode() {
        let states = IndicatorBuilder::new()
            .gse1(false)
            .gse2(true)
            .te_ra(false)
            .te_rb(false)
            .te1(false)
            .te2(false)
            .te3(true)
            .build();

        assert_eq!(states.encode_i2c(), 0b01000010);
    }

    /// Assert that a top bit set results in an error
    #[test]
    fn test_decode_error() {
        let result = IndicatorStates::try_from(0b10000000);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().value, 0b10000000);
    }

    /// Assert that a state encoded and decoded remains the same
    #[test]
    fn test_recode() {
        let states = IndicatorBuilder::new()
            .gse1(false)
            .gse2(true)
            .te_ra(false)
            .te_rb(false)
            .te1(false)
            .te2(false)
            .te3(true)
            .build();

        assert!(states == IndicatorStates::try_from(states.encode_i2c()).unwrap());
    }
}
