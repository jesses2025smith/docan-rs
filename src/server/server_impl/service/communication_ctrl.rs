//! request of Service 28

use crate::server::DoCanServer;
use iso14229_1::{request::Request, DidConfig};

impl<D, C, F> DoCanServer<D, C, F> {
    pub(crate) fn communication_ctrl(&self, req: Request, cfg: &DidConfig) -> Option<Vec<u8>> {
        todo!()
    }
}
