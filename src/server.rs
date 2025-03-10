mod context;

#[cfg(not(feature = "async"))]
mod synchronous;
#[cfg(not(feature = "async"))]
pub use synchronous::*;

pub(crate) mod util;

use rs_can::CanResult;

pub trait Server {
    type Channel;
    type Device;
    type Error;

    fn service_forever(&mut self, interval: u64) -> CanResult<(), Self::Error>;

    fn service_stop(&mut self) -> CanResult<(), Self::Error>;
}
