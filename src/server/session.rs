use iso14229_1::SessionType;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::Mutex, time::interval};

/// Session manager.
#[derive(Debug, Default, Clone)]
pub(crate) struct SessionManager {
    /// current session type
    pub(crate) r#type: Arc<Mutex<SessionType>>,
    /// the start timestamp
    pub(crate) start: Arc<Mutex<Option<Instant>>>,
    /// Keep Duration
    pub(crate) duration: Duration,
    pub(crate) sa_level: Arc<Mutex<u8>>,
    pub(crate) link_ctrl_verified: Arc<Mutex<bool>>,
}

impl SessionManager {
    pub fn new(second: Option<u64>) -> Self {
        Self {
            duration: Duration::from_secs(second.unwrap_or(5)),
            ..Default::default()
        }
    }

    pub async fn reset(&self) {
        self.change(Default::default()).await;
        let _ = self.start.lock().await.take();
        self.set_security_access_level(Default::default()).await;
    }

    /// change session type
    #[inline(always)]
    pub async fn change(&self, r#type: SessionType) {
        let mut guard = self.r#type.lock().await;
        if *guard != r#type {
            self.set_security_access_level(Default::default()).await;
            self.clear_link_control_verify().await;
            if r#type == Default::default() {
                let _ = self.start.lock().await.take();
            }
        }
        *guard = r#type;
    }
    /// Keep session or start non-default session manager
    #[inline(always)]
    pub async fn keep(&self) {
        self.start.lock().await.replace(Instant::now());
    }
    /// get current session type
    #[inline(always)]
    pub async fn set_session_type(&self, r#type: SessionType) {
        self.change(r#type).await;
    }
    /// get current session type
    #[inline(always)]
    pub async fn get_session_type(&self) -> SessionType {
        self.r#type.lock().await.clone()
    }
    #[inline(always)]
    pub async fn set_security_access_level(&self, level: u8) {
        *self.sa_level.lock().await = level;
    }
    #[inline(always)]
    pub async fn get_security_access_level(&self) -> u8 {
        self.sa_level.lock().await.clone()
    }
    #[inline(always)]
    pub async fn arm_link_control_verify(&self) {
        *self.link_ctrl_verified.lock().await = true;
    }
    #[inline(always)]
    pub async fn clear_link_control_verify(&self) {
        *self.link_ctrl_verified.lock().await = false;
    }
    #[inline(always)]
    pub async fn consume_link_control_verify(&self) -> bool {
        let mut guard = self.link_ctrl_verified.lock().await;
        let verified = *guard;
        *guard = false;
        verified
    }
    /// enable task
    pub async fn work(&self) {
        let mut interval = interval(self.duration);

        loop {
            interval.tick().await;

            let mut guard = self.start.lock().await;
            if let Some(non_def_start) = guard.clone() {
                if non_def_start.elapsed() >= self.duration {
                    let _ = guard.take();
                    self.set_session_type(Default::default()).await;
                    self.set_security_access_level(Default::default()).await;
                }
            }
        }
    }
}
