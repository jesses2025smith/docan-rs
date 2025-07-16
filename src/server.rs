mod context;
mod server_impl;
mod tasks;
mod util;

pub use server_impl::*;

use crate::SecurityAlgo;
use iso14229_1::DataIdentifier;
use iso15765_2::Address;

#[async_trait::async_trait]
pub trait Server {
    async fn update_address(&self, address: Address);
    async fn update_security_algo(&self, algo: SecurityAlgo);
    async fn add_data_identifier(&self, did: DataIdentifier, length: usize);
    async fn remove_data_identifier(&self, did: DataIdentifier);
    async fn service_forever(&mut self, interval: u64);

    async fn service_stop(&mut self);
}
