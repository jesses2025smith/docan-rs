use iso14229_1::SessionType;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::Mutex, time::interval};

/// Session manager.
#[derive(Debug, Default, Clone)]
pub struct SessionManager {
    /// current session type
    pub(crate) r#type: Arc<Mutex<SessionType>>,
    /// the start timestamp
    pub(crate) start: Arc<Mutex<Option<Instant>>>,
    /// Keep Duration
    pub(crate) duration: Duration,
    pub(crate) unlocked: Arc<Mutex<bool>>,
}

impl SessionManager {
    pub fn new(second: Option<u64>) -> Self {
        Self {
            duration: Duration::from_secs(second.unwrap_or(5)),
            ..Default::default()
        }
    }

    /// change session type
    #[inline(always)]
    pub async fn change(&mut self, r#type: SessionType) {
        *self.r#type.lock().await = r#type;
    }
    /// Keep session or start non-default session manager
    #[inline(always)]
    pub async fn keep(&self, unlocked: bool) {
        self.start.lock().await.replace(Instant::now());
        *self.unlocked.lock().await = unlocked;
    }
    /// get current session type
    /// if the server is locked, return default session
    /// else return current session
    #[inline(always)]
    pub async fn session_type(&self) -> SessionType {
        if *self.unlocked.lock().await {
            self.r#type.lock().await.clone()
        } else {
            SessionType::default()
        }
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
                    {
                        let mut guard = self.r#type.lock().await;
                        *guard = Default::default();
                        // free `type` lock
                    }
                }
            }
        }
    }
}
