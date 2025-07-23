mod context;
mod server_impl;
mod session;
mod util;

pub use server_impl::*;

use crate::SecurityAlgo;
use iso14229_1::{
    response::SessionTiming, utils::did_config_deserialize, DataIdentifier, DidConfig,
};
use iso15765_2::can::Address;
use rsutil::types::ByteOrder;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

pub type DidSaLevel = HashMap<DataIdentifier, u8>;

fn did_sa_level_deserialize<'de, D>(deserializer: D) -> Result<DidSaLevel, D::Error>
where
    D: Deserializer<'de>,
{
    let raw_map: HashMap<u16, u8> = HashMap::deserialize(deserializer)?;

    let res = raw_map
        .into_iter()
        .map(|(k, v)| (DataIdentifier::from(k), v))
        .collect::<HashMap<_, _>>();

    Ok(res)
}

#[allow(unused)]
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub(crate) address: Address,
    pub(crate) timing: SessionTiming,
    /// extend session security access level
    pub(crate) extend_sa_level: u8,
    /// program session security access level
    pub(crate) program_sa_level: u8,
    pub(crate) seed_len: usize,
    pub(crate) sa_salt: Vec<u8>,
    #[serde(deserialize_with = "did_config_deserialize")]
    pub(crate) did_cfg: DidConfig,
    #[serde(deserialize_with = "did_sa_level_deserialize")]
    pub(crate) did_sa_level: DidSaLevel,
    pub(crate) byte_order: ByteOrder,
}

#[async_trait::async_trait]
pub trait Server {
    async fn update_address(&self, address: Address);
    async fn update_security_algo(&self, algo: SecurityAlgo);
    async fn service_forever(&mut self, interval_us: u64);

    async fn service_stop(&mut self);
}
