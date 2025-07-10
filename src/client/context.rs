use crate::{buffer::IsoTpBuffer, SecurityAlgo};
use bytes::Bytes;
use iso14229_1::DidConfig;
use iso15765_2::{CanIsoTp, IsoTpError, IsoTpEvent, IsoTpEventListener, P2};
use rsutil::types::ByteOrder;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::Mutex, time::sleep};

#[derive(Debug, Default, Clone)]
pub(crate) struct IsoTpListener {
    pub(crate) buffer: IsoTpBuffer,
    pub(crate) p2_ctx: Arc<Mutex<P2>>,
    pub(crate) p2_offset: u64,
}

impl IsoTpListener {
    pub fn new(p2_ctx: P2, p2_offset: u64) -> Self {
        Self {
            buffer: Default::default(),
            p2_ctx: Arc::new(Mutex::new(p2_ctx)),
            p2_offset,
        }
    }
}

impl IsoTpListener {
    pub async fn async_timer(&self, response_pending: bool) -> Result<Bytes, IsoTpError> {
        let p2_ctx = self.p2_ctx.lock().await;
        let tov = if response_pending {
            p2_ctx.p2_star_ms()
        } else {
            p2_ctx.p2_ms() + self.p2_offset
        };

        let timeout = Duration::from_millis(tov);
        let mut start = Instant::now();

        loop {
            sleep(Duration::from_millis(1)).await;

            if start.elapsed() > timeout {
                self.clear_buffer().await;
                return Err(IsoTpError::Timeout {
                    value: tov,
                    unit: "ms",
                });
            }

            match self.buffer_data().await {
                Some(event) => match event {
                    IsoTpEvent::Wait | IsoTpEvent::FirstFrameReceived => {
                        start = Instant::now();
                    }
                    IsoTpEvent::DataReceived(data) => {
                        // rsutil::trace!("DoCAN - data received: {}", hex::encode(&data));
                        return Ok(data);
                    }
                    IsoTpEvent::ErrorOccurred(e) => {
                        self.clear_buffer().await;
                        return Err(e.clone());
                    }
                },
                None => continue,
            }
        }
    }
    #[inline(always)]
    pub async fn update_p2_ctx(&self, p2: u16, p2_star: u32) {
        self.p2_ctx.lock().await.update(p2, p2_star)
    }
}

#[async_trait::async_trait]
impl IsoTpEventListener for IsoTpListener {
    #[inline(always)]
    async fn buffer_data(&self) -> Option<IsoTpEvent> {
        self.buffer.get().await
    }
    #[inline(always)]
    async fn clear_buffer(&self) {
        self.buffer.clear().await;
    }
    #[inline(always)]
    async fn on_iso_tp_event(&self, event: IsoTpEvent) {
        self.buffer.set(event).await
    }
}

#[derive(Clone)]
pub struct Context<C: Clone + Eq, F> {
    pub(crate) iso_tp: CanIsoTp<C, F>,
    pub(crate) listener: IsoTpListener,
    pub(crate) did: Arc<Mutex<DidConfig>>,
    pub(crate) security_algo: Arc<Mutex<Option<SecurityAlgo>>>,
    pub(crate) byte_order: ByteOrder,
}
