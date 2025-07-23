use crate::SecurityAlgo;
use iso14229_1::{response::SessionTiming, DataIdentifier, DidConfig};
use rsutil::types::ByteOrder;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub(crate) struct Context {
    timing: Arc<Mutex<SessionTiming>>,
    did: Arc<Mutex<DidConfig>>,
    security_algo: Arc<Mutex<Option<SecurityAlgo>>>,
    pub(crate) byte_order: ByteOrder,
    pub(crate) p2_offset: u64,
}

impl Context {
    pub fn new(byte_order: ByteOrder, p2_offset: Option<u16>) -> Self {
        Self {
            timing: Default::default(),
            did: Default::default(),
            security_algo: Default::default(),
            byte_order,
            p2_offset: p2_offset.unwrap_or_default() as u64,
        }
    }

    #[inline(always)]
    pub async fn set_session_timing(&self, val: SessionTiming) {
        *self.timing.lock().await = val
    }

    #[inline(always)]
    pub async fn get_session_timing(&self) -> SessionTiming {
        self.timing.lock().await.clone()
    }

    #[inline(always)]
    pub async fn add_did(&self, did: DataIdentifier, size: usize) {
        self.did.lock().await.insert(did, size);
    }

    #[inline(always)]
    pub async fn remove_did(&self, did: &DataIdentifier) {
        self.did.lock().await.remove(did);
    }

    #[inline(always)]
    pub async fn get_did_cfg(&self) -> DidConfig {
        self.did.lock().await.clone()
    }

    #[inline(always)]
    pub async fn set_security_algo(&self, algo: SecurityAlgo) {
        let _ = self.security_algo.lock().await.insert(algo);
    }

    #[inline(always)]
    pub async fn get_security_algo(&self) -> Option<SecurityAlgo> {
        self.security_algo.lock().await.clone()
    }
}
