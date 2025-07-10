use iso15765_2::IsoTpEvent;
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::Mutex;

#[derive(Debug, Default, Clone)]
pub struct IsoTpBuffer {
    inner: Arc<Mutex<VecDeque<IsoTpEvent>>>,
}

impl IsoTpBuffer {
    #[inline(always)]
    pub async fn clear(&self) {
        self.inner.lock().await.clear();
    }

    #[inline(always)]
    pub async fn set(&self, event: IsoTpEvent) {
        self.inner.lock().await.push_back(event);
    }

    #[inline(always)]
    pub async fn get(&self) -> Option<IsoTpEvent> {
        self.inner.lock().await.pop_front()
    }
}
