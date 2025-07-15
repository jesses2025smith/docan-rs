mod context;
mod server_impl;
mod tasks;
mod util;

pub use server_impl::*;

#[async_trait::async_trait]
pub trait Server {
    async fn service_forever(&mut self, interval: u64);

    async fn service_stop(&mut self);
}
