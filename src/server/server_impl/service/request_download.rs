//! request of Service 34

use crate::server::DoCanServer;
use iso14229_1::{request::Request, DidConfig};

impl<D, C, F> DoCanServer<D, C, F> {
    pub(crate) fn request_download(&self, req: Request, cfg: &DidConfig) -> Option<Vec<u8>> {
        todo!()
    }
}
