use std::{fmt::Display, hash::Hash};
use iso15765_2::{Address, CanAdapter, CanIsoTp};
use rs_can::{CanDevice, CanFrame, CanResult};
use crate::{DoCanError, Server};
use super::context::{Context, IsoTpListener};

#[derive(Clone)]
pub struct DoCanServer<D, C, F> {
    adapter: CanAdapter<D, C, F>,
    context: Context<C, F>,
}

impl<D, C, F> DoCanServer<D, C, F>
where
    D: CanDevice<Channel = C, Frame = F> + Clone + Send + 'static,
    C: Display + Clone + Hash + Eq + 'static,
    F: CanFrame<Channel = C> + Clone + Send + Display + 'static
{
    pub fn new(adapter: CanAdapter<D, C, F>, channel: C, address: Address) -> Self {
        let listener = IsoTpListener {
            buffer: Default::default(),
        };
        let iso_tp = CanIsoTp::new(
            channel.clone(),
            address,
            adapter.sender(),
            Box::new(listener.clone()),
        );
        adapter.register_listener(
            format!("DoCANServer-{}", channel),
            Box::new(iso_tp.clone()),
        );
        Self {
            adapter,
            context: Context::new(iso_tp, listener),
        }
    }

    #[inline]
    pub fn adapter(&self) -> &CanAdapter<D, C, F> {
        &self.adapter
    }
}

impl<D, C, F> Server for DoCanServer<D, C, F>
where
    C: Display + Clone,
    F: CanFrame<Channel = C>
{
    type Channel = C;
    type Device = D;
    type Error = DoCanError;

    fn service_forever(&mut self, interval: u64) -> CanResult<(), Self::Error> {
        self.context.server(interval)
    }

    fn service_stop(&mut self) -> CanResult<(), Self::Error> {
        self.context.stop()
    }
}
