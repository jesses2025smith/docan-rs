mod context;
mod server_impl;
mod session;
mod util;

pub use server_impl::*;

use crate::SecurityAlgo;
use iso14229_1::{response::SessionTiming, utils::did_config_deserialize, DidConfig};
use iso15765_2::can::Address;
use rsutil::types::ByteOrder;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub(crate) address: Address,
    pub(crate) timing: SessionTiming,
    /// extend session security access level
    pub(crate) extend_sa_level: u8,
    /// program session security access level
    pub(crate) program_sa_level: u8,
    pub(crate) sa_salt: Vec<u8>,
    #[serde(deserialize_with = "did_config_deserialize")]
    pub(crate) did_cfg: DidConfig,
    pub(crate) byte_order: ByteOrder,
}

#[async_trait::async_trait]
pub trait Server {
    async fn update_address(&self, address: Address);
    async fn update_security_algo(&self, algo: SecurityAlgo);
    async fn service_forever(&mut self, interval_us: u64);

    async fn service_stop(&mut self);
}
