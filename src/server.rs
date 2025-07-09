mod context;
mod server_impl;
pub use server_impl::*;

pub(crate) mod util;

use rs_can::CanResult;

#[async_trait::async_trait]
pub trait Server {
    type Channel;
    type Device;
    type Error;

    async fn service_forever(&mut self, interval: u64) -> CanResult<(), Self::Error>;

    async fn service_stop(&mut self) -> CanResult<(), Self::Error>;
}
