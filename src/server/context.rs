use crate::{server::session::SessionManager, Config, DoCanError, SecurityAlgo};
use bytes::{Bytes, BytesMut};
use iso14229_1::{
    request::ClearDiagnosticInfo, response::SessionTiming, DataIdentifier, DidConfig,
    MemoryLocation,
};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    fs::read,
    sync::{Mutex, MutexGuard},
};

#[derive(Clone)]
pub(crate) struct Context {
    pub(crate) config: Config,
    /// static did
    pub(crate) did_st: Arc<Mutex<HashMap<DataIdentifier, Bytes>>>,
    /// dynamic did
    pub(crate) did_dyn: Arc<Mutex<HashMap<DataIdentifier, Bytes>>>,
    pub(crate) sa_algo: Arc<Mutex<Option<SecurityAlgo>>>,
    pub(crate) sa_ctx: Arc<Mutex<Option<(u8, Bytes)>>>,
    #[allow(dead_code)]
    pub(crate) memories: Arc<Mutex<HashMap<MemoryLocation, Bytes>>>,
    pub(crate) session: SessionManager,
}

impl Context {
    pub async fn new() -> Result<Self, DoCanError> {
        let reader = read("docan.server.yaml")
            .await
            .map_err(|e| DoCanError::OtherError(format!("{:?}", e)))?;
        let config = serde_yaml::from_slice::<Config>(reader.as_slice())
            .map_err(|e| DoCanError::OtherError(format!("{:?}", e)))?;

        Ok(Self {
            config,
            did_st: Default::default(),
            did_dyn: Default::default(),
            sa_algo: Default::default(),
            sa_ctx: Default::default(),
            memories: Default::default(),
            session: Default::default(),
        })
    }

    pub async fn reset(&self) {
        self.did_dyn.lock().await.clear();
        self.session.reset().await;
    }

    #[inline(always)]
    pub fn get_timing(&self) -> &SessionTiming {
        &self.config.timing
    }

    #[inline(always)]
    pub fn get_did_config(&self) -> &DidConfig {
        &self.config.did_cfg
    }

    pub async fn set_static_did<T: AsRef<[u8]>>(&mut self, did: &DataIdentifier, data: T) -> bool {
        match self.config.did_cfg.get(did) {
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

    #[inline(always)]
    pub async fn get_static_did(&self, did: &DataIdentifier) -> Option<Bytes> {
        self.did_get_util(self.did_st.lock().await, &did)
    }

    #[inline(always)]
    pub fn get_static_did_sa_level(&self, did: &DataIdentifier) -> Option<u8> {
        self.config.did_sa_level.get(did).cloned()
    }

    #[allow(unused)]
    #[inline(always)]
    pub async fn set_dynamic_did<T: AsRef<[u8]>>(&mut self, did: &DataIdentifier, data: T) -> bool {
        match self.config.did_cfg.get(did) {
            Some(&len) => {
                let data = data.as_ref();
                if len != data.len() {
                    false
                } else {
                    self.did_dyn
                        .lock()
                        .await
                        .insert(*did, BytesMut::from(data).freeze());
                    true
                }
            }
            None => false,
        }
    }

    #[allow(unused)]
    #[inline(always)]
    pub async fn get_dynamic_did(&self, did: &DataIdentifier) -> Option<Bytes> {
        self.did_get_util(self.did_dyn.lock().await, &did)
    }

    #[inline(always)]
    pub fn get_security_salt(&self) -> &[u8] {
        &self.config.sa_salt
    }

    #[inline(always)]
    pub(crate) async fn set_security_algo(&self, alg: SecurityAlgo) {
        let _ = self.sa_algo.lock().await.replace(alg);
    }

    #[inline(always)]
    pub async fn get_security_algo(&self) -> Option<SecurityAlgo> {
        self.sa_algo.lock().await.clone()
    }

    #[inline(always)]
    fn did_get_util<'a>(
        &self,
        guard: MutexGuard<'a, HashMap<DataIdentifier, Bytes>>,
        did: &DataIdentifier,
    ) -> Option<Bytes> {
        match guard.get(did) {
            Some(data) => Some(data.clone()),
            None => {
                drop(guard);
                match self.config.did_cfg.get(did) {
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

    #[allow(unused)]
    pub(crate) async fn clear_diagnostic_info(&self, info: ClearDiagnosticInfo) {}
}
