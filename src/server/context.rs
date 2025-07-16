use crate::SecurityAlgo;
use bytes::{Bytes, BytesMut};
use iso14229_1::{response::SessionTiming, DataIdentifier, DidConfig};
use rsutil::types::ByteOrder;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Default, Clone)]
pub(crate) struct Context {
    pub(crate) timing: Arc<Mutex<SessionTiming>>,
    pub(crate) did_cfg: Arc<Mutex<DidConfig>>,
    /// static did
    pub(crate) did_st: Arc<Mutex<HashMap<DataIdentifier, Bytes>>>,
    pub(crate) did_dny: Arc<Mutex<HashMap<DataIdentifier, Bytes>>>,
    pub(crate) security_algo: Arc<Mutex<Option<SecurityAlgo>>>,
    pub(crate) byte_order: ByteOrder,
}

impl Context {
    pub fn new(byte_order: ByteOrder) -> Self {
        Self {
            byte_order,
            ..Default::default()
        }
    }

    pub async fn reset(&self) {
        self.did_cfg.lock().await.clear();
        *self.timing.lock().await = Default::default();
    }

    #[inline(always)]
    pub async fn get_timing(&self) -> SessionTiming {
        self.timing.lock().await.clone()
    }

    #[inline(always)]
    pub(crate) async fn set_security_algo(&self, alg: SecurityAlgo) {
        self.security_algo.lock().await.replace(alg);
    }

    #[inline(always)]
    pub(crate) async fn add_did(&self, did: DataIdentifier, size: usize) {
        self.did_cfg.lock().await.insert(did, size);
    }

    #[inline(always)]
    pub(crate) async fn remove_did(&self, did: &DataIdentifier) {
        self.did_cfg.lock().await.remove(did);
    }

    #[inline(always)]
    pub async fn get_did_config(&self) -> DidConfig {
        self.did_cfg.lock().await.clone()
    }

    pub async fn get_static_did(&self, did: &DataIdentifier) -> Option<Bytes> {
        let guard = self.did_st.lock().await;
        match guard.get(did) {
            Some(data) => Some(data.clone()),
            None => {
                drop(guard);
                match self.did_cfg.lock().await.get(did) {
                    Some(&len) => {
                        let mut data = Vec::with_capacity(len);
                        data.resize(len, 0);
                        Some(Bytes::from(data))
                    }
                    None => None,
                }
            }
        }
    }

    pub async fn set_static_did<T: AsRef<[u8]>>(&mut self, did: &DataIdentifier, data: T) -> bool {
        match self.did_cfg.lock().await.get(did) {
            Some(&len) => {
                let data = data.as_ref();
                if len != data.len() {
                    false
                } else {
                    self.did_st
                        .lock()
                        .await
                        .insert(*did, BytesMut::from(data).freeze());
                    true
                }
            }
            None => false,
        }
    }

    pub(crate) async fn clear_diagnostic_info(&self) {

    }
}
