use core::future::Future;
use core::task::{Context, Poll};
use futures::future::{select, Either, FutureExt};

use embedded_hal_async::i2c;
use futures::pin_mut;
use rp235x_hal::async_utils::AsyncPeripheral;

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
            AsyncI2cError::DeviceError(err) => err.kind(),
        }
    }
}

pub struct AsyncI2c<I2C> {
    device: I2C,
    timeout: u32,
}

impl<I2C> AsyncI2c<I2C>
where
    I2C: i2c::I2c + AsyncPeripheral,
{
    pub fn new(device: I2C, timeout: u32) -> Self {
        Self { device, timeout }
    }

    pub fn on_interrupt() {
        I2C::on_interrupt()
    }
}

impl<D> i2c::ErrorType for AsyncI2c<D>
where
    D: i2c::I2c,
{
    type Error = AsyncI2cError<D::Error>;
}

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
            Ok(result) => result.map_err(AsyncI2cError::DeviceError),
            Err(()) => Err(AsyncI2cError::Timeout),
        }
    }
}

pub async fn timeout_future<F, T>(future: F, ticks: u32) -> Result<T, ()>
where
    F: Future<Output = T>,
{
    pin_mut!(future);

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

    match select(future.fuse(), timeout.fuse()).await {
        Either::Left((result, _)) => Ok(result),
        Either::Right((_, _)) => Err(()),
    }
}
