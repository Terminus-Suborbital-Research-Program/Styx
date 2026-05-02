use common_states::indicators::IndicatorStates;
use crate::tasks::pins::{Atmega, IndicatorError};
use crate::gpio::{Pin, read::ReadPin, write::WritePin};

// Trait for controlling GSE / TE interface and battery latch control responsibilities
pub trait BoardHardware {
    fn pins(&mut self) -> Result<IndicatorStates, IndicatorError>;
    fn activate_latch(&mut self);
    fn idle_latch(&mut self);
    fn deactivate_latch(&mut self);
}

impl BoardHardware for Atmega {
    fn pins(&mut self) -> Result<IndicatorStates, IndicatorError> { self.pins() }
    fn activate_latch(&mut self) { self.activate_latch() }
    fn idle_latch(&mut self) { self.idle_latch() }
    fn deactivate_latch(&mut self) { self.deactivate_latch() }
}

// When directly reading through jupiter instead of having an atmega interface
pub struct GpioHardware {
    gse1: ReadPin,
    gse2: ReadPin,
    te_ra: ReadPin,
    te_rb: ReadPin,
    te1: ReadPin,
    te2: ReadPin,
    te3: ReadPin,
    battery_latch: WritePin,
}

impl GpioHardware {
    pub fn new() -> Self {
        Self {
            gse1: Pin::new("GPIO2").into(),
            gse2: Pin::new("GPIO3").into(),
            te_ra: Pin::new("GPIO4").into(),
            te_rb: Pin::new("GPIO5").into(),
            te1: Pin::new("GPIO6").into(),
            te2: Pin::new("GPIO7").into(),
            te3: Pin::new("GPIO8").into(),
            battery_latch: Pin::new("GPIO9").into(),
        }
    }
}

impl BoardHardware for GpioHardware {
    fn pins(&mut self) -> Result<IndicatorStates, IndicatorError> {
        use common_states::indicators::IndicatorBuilder;
        
        // If a pin fails to read, we default to false to prevent crashing
        Ok(IndicatorBuilder::new()
            .gse1(self.gse1.read().unwrap_or(false))
            .gse2(self.gse2.read().unwrap_or(false))
            .te_ra(self.te_ra.read().unwrap_or(false))
            .te_rb(self.te_rb.read().unwrap_or(false))
            .te1(self.te1.read().unwrap_or(false))
            .te2(self.te2.read().unwrap_or(false))
            .te3(self.te3.read().unwrap_or(false))
            .build())
    }

    fn activate_latch(&mut self) { self.battery_latch.write(true).ok(); }
    fn idle_latch(&mut self) {  }
    fn deactivate_latch(&mut self) { self.battery_latch.write(false).ok(); }
}

// Use conditional type alias so atmega and gpio can be switched out with just a config flag.

#[cfg(feature = "legacy_atmega")]
pub type ActiveHardware = Atmega;

#[cfg(not(feature = "legacy_atmega"))]
pub type ActiveHardware = GpioHardware;