use core::future::Future;
use core::task::{Context, Poll};
use futures::future::{select, Either, FutureExt};

use embedded_hal_async::i2c;
use futures::pin_mut;
use rp235x_hal::async_utils::AsyncPeripheral;

/// A generic I2C device that is able to transact without fear of locking the device indefinitely
#[derive(Clone, Copy, PartialEq, Eq, Debug, defmt::Format)]
pub enum AsyncI2cError<T: i2c::Error> {
    Timeout,
    DeviceError(T),
}

impl<E> i2c::Error for AsyncI2cError<E>
where
    E: i2c::Error,
{
    fn kind(&self) -> i2c::ErrorKind {
        match self {
            AsyncI2cError::Timeout => i2c::ErrorKind::ArbitrationLoss,
            AsyncI2cError::DeviceError(e) => e.kind(),
        }
    }
}

/// An asynchronous IO device that can timeout
pub struct AsyncI2c<I2C> {
    device: I2C,
    timeout: u32,
}

impl<I2C> AsyncI2c<I2C>
where
    I2C: i2c::I2c + AsyncPeripheral,
{
    /// Creates a new async I2C device
    pub fn new(device: I2C, timeout: u32) -> Self {
        Self { device, timeout }
    }

    /// Returns the inner, dissasembling
    pub fn into_inner(self) -> I2C {
        self.device
    }

    /// Returns the limit for ticks
    pub fn timeout_ticks(&self) -> u32 {
        self.timeout
    }

    /// Sets the tick limit
    pub fn set_timeout_limit(&mut self, ticks: u32) {
        self.timeout = ticks
    }

    /// Wakes the device
    pub fn on_interrupt() {
        I2C::on_interrupt()
    }
}

/// Inherit the old error type from the device
impl<D> i2c::ErrorType for AsyncI2c<D>
where
    D: i2c::I2c,
{
    type Error = AsyncI2cError<D::Error>;
}

/// Re-impliment the I2C traits by polling every future. Make sure the i2c bus has a wake on it!
impl<D> i2c::I2c for AsyncI2c<D>
where
    D: i2c::I2c,
{
    async fn transaction(
        &mut self,
        address: i2c::SevenBitAddress,
        operations: &mut [i2c::Operation<'_>],
    ) -> Result<(), Self::Error> {
        match timeout_future(self.device.transaction(address, operations), self.timeout).await {
            Ok(transacted) => match transacted {
                Ok(_) => Ok(()),
                Err(e) => Err(AsyncI2cError::DeviceError(e)),
            },

            Err(_) => Err(AsyncI2cError::Timeout),
        }
    }
}

/// Re-impliment the regular i2c trait where D: embedded_hal::i2c::I2c
impl<D> embedded_hal::i2c::I2c for AsyncI2c<D>
where
    D: embedded_hal::i2c::I2c + i2c::I2c,
{
    fn transaction(
        &mut self,
        address: u8,
        operations: &mut [embedded_hal::i2c::Operation],
    ) -> Result<(), Self::Error> {
        match embedded_hal::i2c::I2c::transaction(&mut self.device, address, operations) {
            Ok(()) => Ok(()),
            Err(e) => Err(AsyncI2cError::DeviceError(e)),
        }
    }
}

/// Runs the provided future with a timeout specified by the number of ticks.
/// Returns Ok(result) if the future completes before timing out,
/// or Err(()) if the timeout elapses.
pub async fn timeout_future<F, T>(future: F, ticks: u32) -> Result<T, ()>
where
    F: Future<Output = T>,
{
    // Pin the future on the stack.
    pin_mut!(future);

    // Create a timeout future using a simple counter.
    let timeout = async {
        let mut count = 0;
        futures::future::poll_fn(|cx: &mut Context<'_>| {
            cx.waker().wake_by_ref();
            if count >= ticks {
                Poll::Ready(())
            } else {
                count += 1;
                Poll::Pending
            }
        })
        .await;
    };
    pin_mut!(timeout);

    // Fuse both futures and select whichever completes first.
    match select(future.fuse(), timeout.fuse()).await {
        Either::Left((result, _)) => Ok(result),
        Either::Right((_, _)) => Err(()),
    }
}
