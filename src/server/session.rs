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
    }

    /// change session type
    #[inline(always)]
    pub async fn change(&self, r#type: SessionType) {
        *self.r#type.lock().await = r#type;
    }
    /// Keep session or start non-default session manager
    #[inline(always)]
    pub async fn keep(&self) {
        self.start.lock().await.replace(Instant::now());
    }
    /// get current session type
    #[inline(always)]
    pub async fn set_session_type(&self, r#type: SessionType) {
        *self.r#type.lock().await = r#type;
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
